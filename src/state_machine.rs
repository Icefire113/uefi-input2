extern crate alloc;
use alloc::collections::VecDeque;
use uefi::boot::{get_handle_for_protocol, open_protocol_exclusive, ScopedProtocol};
use uefi::Result;
use crate::key_data::KeyData;
use uefi::proto::console::text::Key;
use uefi::proto::misc::Timestamp;

#[derive(Debug, Clone, Copy)]
pub enum InputEvent {
    Pressed(KeyData),
    Released(KeyData),
    LongPressed(KeyData),
    SingleClick(KeyData),
    DoubleClick(KeyData),
    Repeat(KeyData),
}

#[derive(Debug, Clone, Copy)]
enum State {
    Idle,
    Pressed {
        key: KeyData,
        start_time: u64,
        last_seen_time: u64,
        long_press_triggered: bool,
        next_repeat_time: u64,
    },
    WaitingForDouble {
        key: KeyData,
        release_time: u64,
    },
}

pub struct StateMachine {
    timestamp: ScopedProtocol<Timestamp>,
    frequency: u64,

    // Config (ticks)
    release_timeout: u64,
    long_press_delay: u64,
    double_click_window: u64,
    repeat_delay: u64,

    state: State,
    event_queue: VecDeque<InputEvent>,
}

fn key_eq(a: &KeyData, b: &KeyData) -> bool {
    let key_match = match (a.key, b.key) {
        (Key::Printable(c1), Key::Printable(c2)) => c1 == c2,
        (Key::Special(s1), Key::Special(s2)) => s1 == s2,
        _ => false,
    };
    if !key_match { return false; }

    a.key_state.key_shift_state == b.key_state.key_shift_state &&
    a.key_state.key_toggle_state == b.key_state.key_toggle_state
}

impl StateMachine {
    pub fn new() -> Result<Self> {
        let timestamp = get_handle_for_protocol::<Timestamp>()?;

        let timestamp = open_protocol_exclusive::<Timestamp>(timestamp)?;

        let props = timestamp.get_properties()?;

        let freq = props.frequency;

        Ok(Self {
            timestamp,
            frequency: freq,
            release_timeout: (freq * 200) / 1000,
            long_press_delay: (freq * 500) / 1000,
            double_click_window: (freq * 300) / 1000,
            repeat_delay: (freq * 100) / 1000,
            state: State::Idle,
            event_queue: VecDeque::new(),
        })
    }

    pub fn update(&mut self, current_key: Option<KeyData>) -> Option<InputEvent> {
        // 优先处理队列里的存量事件
        if let Some(event) = self.event_queue.pop_front() {
            return Some(event);
        }

        let now = self.timestamp.get_timestamp();

        match self.state {
            State::Idle => {
                if let Some(key) = current_key {
                    self.state = State::Pressed {
                        key,
                        start_time: now,
                        last_seen_time: now,
                        long_press_triggered: false,
                        next_repeat_time: now + self.long_press_delay,
                    };
                    return Some(InputEvent::Pressed(key));
                }
            }

            State::Pressed { key, start_time, last_seen_time, long_press_triggered, next_repeat_time } => {
                if let Some(curr) = current_key {
                    if key_eq(&key, &curr) {
                        // 【持续按压中】
                        let mut long_triggered = long_press_triggered;
                        let mut next_repeat = next_repeat_time;

                        if !long_triggered && now.saturating_sub(start_time) > self.long_press_delay {
                            long_triggered = true;
                            next_repeat = now + self.repeat_delay;
                            self.event_queue.push_back(InputEvent::LongPressed(key));
                        } else if long_triggered && now > next_repeat {
                            next_repeat = now + self.repeat_delay;
                            self.event_queue.push_back(InputEvent::Repeat(key));
                        }

                        self.state = State::Pressed {
                            key,
                            start_time,
                            last_seen_time: now, // 刷新“最后可见时间”
                            long_press_triggered: long_triggered,
                            next_repeat_time: next_repeat,
                        };
                    } else {
                        // 【突然换键】先释放旧的，状态转为 Idle，下一帧自然会处理新键
                        self.state = State::Idle;
                        self.event_queue.push_back(InputEvent::Released(key));
                        if !long_press_triggered {
                            self.event_queue.push_back(InputEvent::SingleClick(key));
                        }
                        // 这里不立即递归，防止死循环
                    }
                } else {
                    // 【读到 None】检查是否真的松开了（超过 release_timeout）
                    if now.saturating_sub(last_seen_time) > self.release_timeout {
                        self.event_queue.push_back(InputEvent::Released(key));
                        if long_press_triggered {
                            self.state = State::Idle;
                        } else {
                            self.state = State::WaitingForDouble {
                                key,
                                release_time: now,
                            };
                        }
                    }
                }
            }

            State::WaitingForDouble { key, release_time } => {
                if let Some(curr) = current_key {
                    if key_eq(&key, &curr) {
                        // 【双击成功】
                        if now.saturating_sub(release_time) <= self.double_click_window {
                            self.state = State::Pressed {
                                key: curr,
                                start_time: now,
                                last_seen_time: now,
                                long_press_triggered: false,
                                next_repeat_time: now + self.long_press_delay,
                            };
                            self.event_queue.push_back(InputEvent::Pressed(curr));
                            return Some(InputEvent::DoubleClick(curr));
                        }
                    }
                    // 如果按了不同的键，或者超时了
                    self.event_queue.push_back(InputEvent::SingleClick(key));
                    self.state = State::Idle;
                } else if now.saturating_sub(release_time) > self.double_click_window {
                    // 【等待超时】确认是单击
                    self.state = State::Idle;
                    return Some(InputEvent::SingleClick(key));
                }
            }
        }

        self.event_queue.pop_front()
    }
}
