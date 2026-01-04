// Copyright (c) Bemly, January 2026
// You may copy and distribute this file freely.  Any queries and
// complaints should be forwarded to bemly_@petalmail.com.
// If you make any changes to this file, please do not distribute
// the results under the name `bemly'.

extern crate alloc;
use alloc::collections::VecDeque;
use core::time::Duration;
use uefi::boot::stall;
use uefi::proto::console::text::Key::Printable;
use uefi::data_types::chars::NUL_16;
use crate::config::{click_window, long_press_delay, release_timeout};
use crate::key_data::KeyData;
use crate::state_machine::{InputEvent, State};
use crate::timer_tick;

/// A timing-robust input state machine that operates independently of UEFI protocols.
///
/// `StateMachineFallback` uses the CPU's Time Stamp Counter (TSC) and an initial
/// calibration phase to determine elapsed time. It handles raw key transitions
/// and converts them into semantic events like `LongPressed`, `Click`, and `Repeat`.
pub struct StateMachineFallback {
    /// The measured frequency of the hardware timer (ticks per second).
    timer_freq: f64,

    /// Maximum duration allowed between hardware signals before a key is considered released.
    release_timeout: Duration,

    /// Duration a key must be held to trigger a `LongPressed` event.
    long_press_delay: Duration,

    /// Maximum gap between a release and a press to count as a multi-click (e.g., double-click).
    click_window: Duration,

    /// Current internal logic state (Idle, Active, or Cooldown).
    state: State,

    /// Buffer for generated events to be consumed by the caller.
    event_queue: VecDeque<InputEvent>,
}

impl StateMachineFallback {
    /// Calibrates the CPU tick frequency by correlating `timer_tick` with UEFI `stall`.
    ///
    /// This performs a 100ms delay to sample the TSC delta and extrapolates the
    /// per-second frequency.
    fn calibrate_ticks() -> u64 {
        let start = timer_tick();
        stall(Duration::from_millis(100));
        let end = timer_tick();
        (end - start) * 10
    }

    /// Creates a new `StateMachineFallback` instance and performs hardware timing calibration.
    ///
    /// This function will block for approximately 100ms during the calibration phase.
    pub fn new() -> Self {
        Self {
            timer_freq: Self::calibrate_ticks() as f64,

            release_timeout: Duration::from_millis(release_timeout()),
            long_press_delay: Duration::from_millis(long_press_delay()),
            click_window: Duration::from_millis(click_window()),

            state: State::Idle,
            event_queue: VecDeque::new(),
        }

    }

    /// Updates the state machine with the current keyboard input and returns the next pending event.
    ///
    /// # Arguments
    /// * `current_key` - The `KeyData` currently reported by the hardware, or `None` if no key is pressed.
    ///
    /// # Returns
    /// * `Some(InputEvent)` if a new event (like a Click or LongPress) has been detected.
    /// * `None` if no new event is available or if the input is empty.
    pub fn update(&mut self, current_key: Option<KeyData>) -> Option<InputEvent> {
        if let Some(event) = self.event_queue.pop_front() { return Some(event) }
        if let Some(KeyData { key: Printable(NUL_16), .. }) = current_key { return None }

        let now_tick = timer_tick();

        // extract fields in advance to remove the dependency on self.
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
                start_time: start_tick,
                last_seen_time: last_seen_tick,
                long_press_triggered,
                click_count
            } => {
                match current_key {
                    Some(curr) if &curr == key => {
                        *last_seen_tick = now_tick;

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
                    },
                    Some(curr) => {
                        let (k, lp, c) = (*key, *long_press_triggered, *click_count);
                        self.emit_release_and_click(&k, lp, c);
                        self.state = State::Idle;
                        self.enter_active(curr, now_tick, 1);
                    },
                    None if Duration::from_secs_f64(now_tick.saturating_sub(*last_seen_tick) as f64 / freq) > timeout => {
                        let (k, lp, c) = (*key, *long_press_triggered, *click_count);
                        self.emit_release_and_click(&k, lp, c);

                        self.state = if !lp {
                            State::Cooldown { key: k, release_time: now_tick, click_count: c }
                        } else {
                            State::Idle
                        }
                    },
                    None => {},
                }
            },

            State::Cooldown {
                key, release_time: release_tick, click_count
            } => {
                let elapsed = Duration::from_secs_f64(
                    now_tick.saturating_sub(*release_tick) as f64 / freq
                );

                match current_key {
                    Some(curr) if curr == *key && elapsed < window => {
                        let count = *click_count + 1;
                        self.state = State::Idle;
                        self.enter_active(curr, now_tick, count);
                    },
                    Some(curr) => {
                        self.state = State::Idle;
                        return self.update(Some(curr));
                    },
                    None if elapsed > window => {
                        self.state = State::Idle;
                    },
                    _ => {},
                }
            },
        }

        self.event_queue.pop_front()
    }
    /// Transition the state machine into the `Active` state and record the initial press.
    #[inline(always)]
    fn enter_active(&mut self, key: KeyData, now_tick: u64, count: usize) {
        self.state = State::Active {
            key,
            start_time: now_tick,
            last_seen_time: now_tick,
            long_press_triggered: false,
            click_count: count,
        };
        self.event_queue.push_back(InputEvent::Pressed(key));
    }

    /// Queues a `Released` event and, if the key was not long-pressed, a `Click` event.
    #[inline(always)]
    fn emit_release_and_click(&mut self, key: &KeyData, lp_triggered: bool, count: usize) {
        self.event_queue.push_back(InputEvent::Released(*key));
        if !lp_triggered {
            self.event_queue.push_back(InputEvent::Click(*key, count));
        }
    }
}