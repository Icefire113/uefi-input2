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
    Repeat(KeyData),
    Click(KeyData, u32),
}

#[derive(Debug, Clone)]
enum State {
    Idle,
    // 正在被物理按下
    Active {
        key: KeyData,
        start_time: u64,
        last_seen_time: u64,
        long_press_triggered: bool,
        next_repeat_time: u64,
        click_count: u32, // 承接自上一次点击
    },
    // 物理已松开，等待可能的下一次点击以构成双击/多击
    Cooldown {
        key: KeyData,
        release_time: u64,
        click_count: u32,
    },
}

pub struct StateMachine {
    timestamp: ScopedProtocol<Timestamp>,
    frequency: u64,

    // 配置
    release_timeout: u64,     // 判定抬手的阈值 (建议 100-150ms)
    long_press_delay: u64,    // 触发长按的时间
    click_window: u64,        // 连击等待窗口 (双击间隔)
    repeat_delay: u64,        // 长按后的重复频率

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
        // 1. 获取 Timestamp 协议句柄
        let timestamp_handle = get_handle_for_protocol::<Timestamp>()?;

        // 2. 以排他模式打开协议（确保我们可以稳定读取硬件计数器）
        let timestamp = open_protocol_exclusive::<Timestamp>(timestamp_handle)?;

        // 3. 获取硬件频率信息
        let props = timestamp.get_properties()?;
        let freq = props.frequency;

        Ok(Self {
            timestamp,
            frequency: freq,

            // --- 核心时间配置 ---

            // 判定抬手超时：建议 150-200ms。
            // 必须大于键盘在按住时发送两个 KeyData 之间的最大间隔。
            release_timeout: (freq * 200) / 1000,

            // 触发长按所需时间：建议 500ms。
            long_press_delay: (freq * 500) / 1000,

            // 连击窗口（双击/三击判定）：建议 250-300ms。
            // 两次点击（松开到下一次按下）超过这个时间，连击计数器重置。
            click_window: (freq * 300) / 1000,

            // 长按触发后的字符重复频率：建议 100ms (即每秒 10 次)。
            repeat_delay: (freq * 100) / 1000,

            state: State::Idle,
            event_queue: VecDeque::new(),
        })
    }

    pub fn update(&mut self, current_key: Option<KeyData>) -> Option<InputEvent> {
        // 1. 队列存量优先
        if let Some(event) = self.event_queue.pop_front() {
            return Some(event);
        }

        let now = self.timestamp.get_timestamp();

        match self.state.clone() {
            State::Idle => {
                if let Some(key) = current_key {
                    self.enter_active(key, now, 1);
                }
            }

            State::Active { key, start_time, last_seen_time, long_press_triggered, next_repeat_time, click_count } => {
                if let Some(curr) = current_key {
                    if key_eq(&key, &curr) {
                        // 【持续按下】更新 last_seen，检查长按
                        let mut next_repeat = next_repeat_time;
                        let mut lp_triggered = long_press_triggered;

                        if !lp_triggered && now.saturating_sub(start_time) > self.long_press_delay {
                            lp_triggered = true;
                            next_repeat = now + self.repeat_delay;
                            self.event_queue.push_back(InputEvent::LongPressed(key));
                        } else if lp_triggered && now >= next_repeat {
                            next_repeat = now + self.repeat_delay;
                            self.event_queue.push_back(InputEvent::Repeat(key));
                        }

                        self.state = State::Active {
                            key,
                            start_time,
                            last_seen_time: now, // 刷新心跳
                            long_press_triggered: lp_triggered,
                            next_repeat_time: next_repeat,
                            click_count,
                        };
                    } else {
                        // 【换键】强制结算旧键，开始新键
                        self.emit_release_and_click(&key, long_press_triggered, click_count);
                        self.enter_active(curr, now, 1);
                    }
                } else {
                    // 【无信号】检查是否心跳超时（视为抬手）
                    if now.saturating_sub(last_seen_time) > self.release_timeout {
                        self.emit_release_and_click(&key, long_press_triggered, click_count);

                        // 只有没触发长按时，才进入连击判定期
                        if !long_press_triggered {
                            self.state = State::Cooldown {
                                key,
                                release_time: now,
                                click_count,
                            };
                        } else {
                            self.state = State::Idle;
                        }
                    }
                }
            }

            State::Cooldown { key, release_time, click_count } => {
                if let Some(curr) = current_key {
                    if key_eq(&key, &curr) && now.saturating_sub(release_time) < self.click_window {
                        // 【连击成功】进入 Active，累加点击数
                        self.enter_active(curr, now, click_count + 1);
                    } else {
                        // 【按了别的键】或【连击窗口已过】
                        self.state = State::Idle;
                        return self.update(Some(curr));
                    }
                } else if now.saturating_sub(release_time) > self.click_window {
                    // 【判定窗口关闭】回到空闲
                    self.state = State::Idle;
                }
            }
        }

        self.event_queue.pop_front()
    }

    // 内部辅助：进入按下状态
    fn enter_active(&mut self, key: KeyData, now: u64, count: u32) {
        self.state = State::Active {
            key,
            start_time: now,
            last_seen_time: now,
            long_press_triggered: false,
            next_repeat_time: now + self.long_press_delay,
            click_count: count,
        };
        self.event_queue.push_back(InputEvent::Pressed(key));
    }

    // 内部辅助：发出 Released 和 Click 事件
    fn emit_release_and_click(&mut self, key: &KeyData, lp_triggered: bool, count: u32) {
        self.event_queue.push_back(InputEvent::Released(*key));
        // 如果长按触发了，就不再产生 Click 事件（长按逻辑终结了点击序列）
        if !lp_triggered {
            self.event_queue.push_back(InputEvent::Click(*key, count));
        }
    }
}
