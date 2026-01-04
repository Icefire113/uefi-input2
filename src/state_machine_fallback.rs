extern crate alloc;
use alloc::collections::VecDeque;
use core::time::Duration;
use uefi::boot::stall;
use uefi::proto::console::text::Key::Printable;
use uefi::data_types::chars::NUL_16;
use crate::key_data::KeyData;
use crate::state_machine::InputEvent;

// 辅助计时函数
fn timer_tick() -> u64 {
    #[cfg(target_arch = "x86")]
    unsafe { core::arch::x86::_rdtsc() }

    #[cfg(target_arch = "x86_64")]
    unsafe { core::arch::x86_64::_rdtsc() }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        let ticks: u64;
        core::arch::asm!("mrs {}, cntvct_el0", out(reg) ticks);
        ticks
    }
}

#[derive(Debug)]
enum State {
    Idle,
    Active {
        key: KeyData,
        start_tick: u64,
        last_seen_tick: u64,
        long_press_triggered: bool,
        click_count: usize,
    },
    Cooldown {
        key: KeyData,
        release_tick: u64,
        click_count: usize,
    },
}

pub struct StateMachineFallback {
    // 存储 CPU 每秒的滴答数 (Frequency)
    timer_freq: f64,

    // 使用 Duration 存储阈值，逻辑更清晰
    release_timeout: Duration,
    long_press_delay: Duration,
    click_window: Duration,

    state: State,
    event_queue: VecDeque<InputEvent>,
}

impl StateMachineFallback {
    fn calibrate_ticks() -> u64 {
        let start = timer_tick();
        stall(Duration::from_millis(100));
        let end = timer_tick();
        (end - start) * 10
    }

    /// 初始化状态机
    /// timer_freq: 硬件计时器的频率（每秒多少个 tick）
    /// 在 UEFI 中可以通过计算两个时钟中断间的 RDTSC 差值获取，或从协议获取。
    pub fn new() -> Self {
        Self {
            timer_freq: Self::calibrate_ticks() as f64,

            release_timeout: Duration::from_millis(150),
            long_press_delay: Duration::from_millis(500),
            click_window: Duration::from_millis(300),

            state: State::Idle,
            event_queue: VecDeque::new(),
        }
    }

    pub fn update(&mut self, current_key: Option<KeyData>) -> Option<InputEvent> {
        if let Some(event) = self.event_queue.pop_front() { return Some(event) }
        if let Some(KeyData { key: Printable(NUL_16), .. }) = current_key { return None }

        let now_tick = timer_tick();

        // --- 关键修复：提前提取字段，解除对 self 的依赖 ---
        let freq = self.timer_freq;
        let timeout = self.release_timeout;
        let lp_delay = self.long_press_delay;
        let window = self.click_window;

        match &mut self.state {
            State::Idle => {
                if let Some(key) = current_key {
                    self.enter_active(key, now_tick, 1);
                }
            },

            State::Active {
                key,
                start_tick,
                last_seen_tick,
                long_press_triggered,
                click_count
            } => {
                match current_key {
                    Some(curr) if &curr == key => {
                        *last_seen_tick = now_tick;

                        // 内联计算时长，不再调用 self.duration_between
                        let held_duration = Duration::from_secs_f64(
                            now_tick.saturating_sub(*start_tick) as f64 / freq
                        );

                        if !*long_press_triggered && held_duration > lp_delay {
                            *long_press_triggered = true;
                            self.event_queue.push_back(InputEvent::LongPressed(*key));
                        } else if *long_press_triggered {
                            let should_push = match self.event_queue.back() {
                                Some(InputEvent::Repeat(last_key)) if last_key == key => false,
                                _ => true,
                            };

                            if should_push {
                                self.event_queue.push_back(InputEvent::Repeat(*key));
                            }
                        }
                    }

                    Some(curr) => {
                        let (k, lp, c) = (*key, *long_press_triggered, *click_count);
                        self.emit_release_and_click(&k, lp, c);
                        self.state = State::Idle;
                        self.enter_active(curr, now_tick, 1);
                    }

                    // 使用提前提取的 timeout 和 freq
                    None if Duration::from_secs_f64(now_tick.saturating_sub(*last_seen_tick) as f64 / freq) > timeout => {
                        let (k, lp, c) = (*key, *long_press_triggered, *click_count);
                        self.emit_release_and_click(&k, lp, c);

                        self.state = if !lp {
                            State::Cooldown { key: k, release_tick: now_tick, click_count: c }
                        } else {
                            State::Idle
                        }
                    }
                    None => {}
                }
            }

            State::Cooldown {
                key, release_tick, click_count
            } => {
                // 使用提前提取的 window 和 freq
                let elapsed = Duration::from_secs_f64(
                    now_tick.saturating_sub(*release_tick) as f64 / freq
                );

                match current_key {
                    Some(curr) if curr == *key && elapsed < window => {
                        let count = *click_count + 1;
                        self.state = State::Idle;
                        self.enter_active(curr, now_tick, count);
                    }
                    Some(curr) => {
                        self.state = State::Idle;
                        return self.update(Some(curr));
                    }
                    None if elapsed > window => {
                        self.state = State::Idle;
                    }
                    _ => {}
                }
            }
        }

        self.event_queue.pop_front()
    }

    #[inline]
    fn enter_active(&mut self, key: KeyData, now_tick: u64, count: usize) {
        self.state = State::Active {
            key,
            start_tick: now_tick,
            last_seen_tick: now_tick,
            long_press_triggered: false,
            click_count: count,
        };
        self.event_queue.push_back(InputEvent::Pressed(key));
    }

    #[inline(always)]
    fn emit_release_and_click(&mut self, key: &KeyData, lp_triggered: bool, count: usize) {
        self.event_queue.push_back(InputEvent::Released(*key));
        if !lp_triggered {
            self.event_queue.push_back(InputEvent::Click(*key, count));
        }
    }
}