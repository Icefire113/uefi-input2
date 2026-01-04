// Copyright (c) Bemly, January 2026
// You may copy and distribute this file freely.  Any queries and
// complaints should be forwarded to bemly_@petalmail.com.
// If you make any changes to this file, please do not distribute
// the results under the name `bemly'.

//! A high-level input state machine that transforms raw hardware signals into semantic events.
//!
//! The `StateMachine` uses a frequency-aware hardware timestamp provider to distinguish
//! between short presses, long presses, and multi-click sequences. It maintains an
//! internal event queue to ensure logical event ordering (e.g., ensuring a `Released`
//! event precedes a `Click` event).

extern crate alloc;
use alloc::collections::VecDeque;
use uefi::boot::{get_handle_for_protocol, open_protocol_exclusive, ScopedProtocol};
use uefi::proto::console::text::Key::Printable;
use uefi::Result;
use uefi::data_types::chars::NUL_16;
use crate::key_data::KeyData;
use uefi::proto::misc::Timestamp;
use crate::config::{click_window, long_press_delay, release_timeout};
use crate::state_machine_fallback::StateMachineFallback;

/// Represents logical input events identified by the state machine.
///
/// Unlike raw hardware signals, `InputEvent` carries temporal and semantic meaning
/// such as long presses and multi-click counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEvent {

    /// Triggered the exact moment a key is physically pressed down.
    ///
    /// **Timing**: Dispatched once when the state transitions from 'Idle/Cooldown' to 'Active'.
    Pressed(KeyData),

    /// Triggered when a key is considered released.
    ///
    /// **Note**: This is emitted after the `release_timeout` has passed without
    /// receiving further signals for this key, ensuring the release is intentional.
    Released(KeyData),

    /// #### Long-Press & Initial Delay
    /// The "Initial Delay" is the period between the first [`InputEvent::LongPressed`]
    /// and the subsequent [`InputEvent::Repeat`] stream. In this implementation:
    /// 1. When `held_duration > long_press_delay`, a `LongPressed` event is fired.
    /// 2. Immediately after (in the same or next tick), the state machine enters
    ///    "Repeat Mode", producing `Repeat` events as long as the key is held.
    ///
    /// #### Event Sequence Timing
    /// Below is the typical lifecycle of a multi-click sequence followed by a hold:
    ///
    /// ```text
    /// Timeline: 0ms       100ms      200ms      300ms      800ms      900ms
    /// Action:   PRESS  -> RELEASE -> PRESS  -> HOLD     -> (Wait)  -> RELEASE
    ///           |          |          |          |          |          |
    /// State:    Idle    -> Active  -> Cooldown-> Active  -> Active  -> Idle
    ///           |          |          |          |          |          |
    /// Events:   Pressed -> Released-> (None)  -> Pressed -> LongPress-> Released
    ///                      Click(1)                         Repeat...
    /// ```
    ///
    /// **Full Sequence Breakdown:**
    /// 1. **Pressed**: Initial physical press detected.
    /// 2. **Released**: Key let go within a short duration.
    /// 3. **Click(1)**: Emitted after `Released` as part of the click detection.
    /// 4. **Cooldown**: The machine waits for `click_window` to see if a second click follows.
    /// 5. **Pressed (2nd)**: Key pressed again within `click_window`, incrementing `click_count`.
    /// 6. **LongPressed**: The 2nd press is held longer than `long_press_delay`.
    /// 7. **Repeat**: Periodic events generated while the key remains held.
    /// 8. **Released & Click(2)**: Final cleanup when the key is finally released.
    LongPressed(KeyData),

    /// Dispatched continuously after a `LongPressed` event as long as the key remains held.
    ///
    /// **Throttling**: The state machine implements a coalescing mechanism. If a `Repeat`
    /// for the same key is already pending in the queue, new ones are suppressed to
    /// prevent input lag during heavy rendering (GOP) tasks.
    Repeat(KeyData),

    /// Triggered upon key release if the press duration did not reach the "Long Press" threshold.
    ///
    /// **Fields**:
    /// * `KeyData`: The metadata of the key.
    /// * `u32`: The click sequence count (1 for single click, 2 for double click, etc.).
    ///
    /// **Timing**: Dispatched immediately after the `Released` event.
    Click(KeyData, usize),
}

/// Represents the internal lifecycle stages of a key tracking operation.
#[derive(Debug)]
pub(crate) enum State {
    /// No keys are currently being tracked. The system is waiting for an initial press.
    Idle,

    /// A key is currently being physically held down.
    ///
    /// The state machine remains in this variant as long as the hardware continues
    /// to report the key or until the `release_timeout` is reached.
    Active {
        /// Metadata of the key being pressed.
        key: KeyData,
        /// The timestamp (in hardware ticks) when the key was first pressed.
        /// Used to calculate the `LongPressed` threshold.
        start_time: u64,
        /// The timestamp of the most recent hardware signal received for this key.
        /// Used as a "heartbeat" to detect when the user has released the key.
        last_seen_time: u64,
        /// Flag indicating if the `LongPressed` event has already been dispatched
        /// for the current press cycle. Prevents duplicate long-press triggers.
        long_press_triggered: bool,
        /// The current click sequence number (e.g., 1 for single press, 2 for double).
        /// This is carried over from a previous `Cooldown` state if the press
        /// happened within the allowed window.
        click_count: usize,
    },

    /// The key has been physically released, but the state machine is "waiting"
    /// to see if the user will press it again to form a double or multi-click.
    ///
    /// If a new press occurs before `click_window` expires, it increments the count.
    /// If the window expires, the state transitions back to `Idle`.
    Cooldown {
        /// Metadata of the key that was just released.
        key: KeyData,
        /// The timestamp when the key was officially considered released.
        release_time: u64,
        /// The click count reached at the end of the last `Active` session.
        click_count: usize,
    },
}

/// A high-level input state machine that transforms raw hardware signals into semantic events.
pub struct StateMachine {
    /// The hardware timestamp protocol used to fetch high-resolution time ticks.
    /// This is used to calculate durations for timeouts and delays.
    timestamp: Option<ScopedProtocol<Timestamp>>,
    fallback: Option<StateMachineFallback>,

    /// The threshold duration used to determine if a key has been physically released.
    /// Since UEFI might not provide an explicit "release" interrupt, the state machine
    /// considers a key released if no signals are received within this window.
    ///
    /// Typical value: 150ms - 200ms.
    release_timeout: u64,
    /// The required duration of a continuous press before a `LongPressed` event is triggered.
    /// After this threshold, the state machine will begin emitting `Repeat` events.
    ///
    /// Typical value: 500ms.
    long_press_delay: u64,
    /// The maximum time window between a key release and the next press to be
    /// considered a multi-click (e.g., a double-click).
    ///
    /// Typical value: 250ms - 300ms.
    click_window: u64,

    /// The current internal tracking state (Idle, Active, or Cooldown).
    state: State,

    /// A buffer of semantic events (e.g., Pressed, Click) waiting to be consumed
    /// by the application's main loop.
    event_queue: VecDeque<InputEvent>,
}

impl StateMachine {

    /// Creates a new `StateMachine` instance by initializing the UEFI Timestamp protocol.
    ///
    /// #### Errors
    /// Returns an error if the `Timestamp` protocol is unavailable or if exclusive
    /// access cannot be granted (e.g., another driver is using it).
    ///
    /// #### Timing Calibration
    /// The input thresholds are automatically calculated based on the hardware frequency:
    /// * **Release Timeout**: 150-200ms. Used to detect key lift when hardware signals stop.
    /// * **Long Press Delay**: 500ms. The duration before a hold is considered a long press.
    /// * **Click Window**: 300ms. The maximum gap allowed between clicks for multi-click detection.
    pub fn new() -> Result<Self> {
        let timestamp = get_handle_for_protocol::<Timestamp>()
            .and_then(|h| open_protocol_exclusive::<Timestamp>(h))
            .ok();

        match timestamp {
            Some(timestamp) => {
                // get timestamp frequency and step (timestamp may overflow).
                let props = timestamp.get_properties()?;
                let freq = props.frequency;

                Ok(Self {
                    timestamp: Some(timestamp),
                    fallback: None,
                    // --- Core Timing Configuration ---

                    // Must be greater than the maximum interval between two KeyData repeats from hardware.
                    release_timeout: (freq * release_timeout()) / 1000,
                    long_press_delay: (freq * long_press_delay()) / 1000,
                    // If the time between release and next press exceeds this, the counter resets.
                    click_window: (freq * click_window()) / 1000,

                    state: State::Idle,
                    event_queue: VecDeque::new(),
                })

            },
            None => {
                Ok(Self {
                    timestamp: None,
                    fallback: Some(StateMachineFallback::new()),
                    release_timeout: 0,
                    long_press_delay: 0,
                    click_window: 0,
                    state: State::Idle,
                    event_queue: VecDeque::new(),
                })
            }
        }
    }

    /// Updates the state machine with the current hardware key status and returns the next logical event.
    ///
    /// This is the core driving method of the input system. It should be called frequently
    /// (typically once per main loop iteration) to ensure accurate timing for long-press
    /// and multi-click detection.
    ///
    /// #### Execution Flow
    /// 1. **Queue Priority**: If events are already pending in the internal buffer (e.g., a `Click`
    ///    generated in the previous cycle), the oldest event is returned immediately.
    /// 2. **Modifier Guard**: Ignores `NUL_16` characters (often sent when only modifiers like
    ///    Shift/Ctrl are pressed) to prevent interrupting active keystrokes.
    /// 3. **State Logic**:
    ///    - **Idle**: Transitions to `Active` on the first valid key press.
    ///    - **Active**: Tracks hold duration for `LongPressed` and `Repeat` events. Detects
    ///      key switches and timeouts (physical releases).
    ///    - **Cooldown**: Waits for a potential second press of the same key to increment
    ///      the click counter (multi-click logic).
    ///
    /// #### Returns
    /// - `Some(InputEvent)`: The next semantic event in the sequence.
    /// - `None`: If no new event was generated or if the input was filtered.
    pub fn update(&mut self, current_key: Option<KeyData>) -> Option<InputEvent> {
        // queue storage priority
        // ensure the state machine sequence is executed
        if let Some(event) = self.event_queue.pop_front() { return Some(event) }

        // modifier key guard (filter realtime mode).
        if let Some(KeyData { key: Printable(NUL_16), .. }) = current_key { return None }

        // Check if we are running in compatibility mode.
        // If so, delegate the update logic to the fallback handler and return early.
        if let Some(ref mut fallback) = self.fallback {
            return fallback.update(current_key)
        }

        // Attempt to retrieve the current hardware timestamp via the UEFI protocol.
        let now = self.timestamp.as_ref()?.get_timestamp();

        match &mut self.state {
            State::Idle => {
                // idle -> active
                if let Some(key) = current_key {
                    self.enter_active(key, now, 1);
                }
                // idle -> idle
            },

            State::Active {
                key,
                start_time,
                last_seen_time,
                long_press_triggered,
                click_count
            } => {
                match current_key {
                    // 1. continuous press of the same key
                    Some(curr) if &curr == key => {
                        *last_seen_time = now;
                        let held_duration = now.saturating_sub(*start_time);
                        if !*long_press_triggered && held_duration > self.long_press_delay {
                            *long_press_triggered = true;
                            self.event_queue.push_back(InputEvent::LongPressed(*key));
                        } else if *long_press_triggered {
                            // Throttling: Only queue Repeat if the queue is empty or the last event is different.
                            if !self.event_queue.back().is_some_and(|e| matches!(e, InputEvent::Repeat(k) if k == key)) {
                                self.event_queue.push_back(InputEvent::Repeat(*key));
                            }
                        }
                    },

                    // 2. key switched (pressed a different key without releasing)
                    Some(curr) => {
                        let (k, lp, c) = (*key, *long_press_triggered, *click_count);
                        self.emit_release_and_click(&k, lp, c);

                        // reset to idle first to clear the mutable borrow of the current state,
                        // then enter active for the new key.
                        self.state = State::Idle;
                        self.enter_active(curr, now, 1);
                    },

                    // 3. check for heartbeat timeout
                    None if now.saturating_sub(*last_seen_time) > self.release_timeout => {
                        let (k, lp, c) = (*key, *long_press_triggered, *click_count);
                        self.emit_release_and_click(&k, lp, c);

                        self.state = if !lp {
                            State::Cooldown { key: k, release_time: now, click_count: c }
                        } else {
                            State::Idle
                        }
                    },

                    // 4. No key signal but within the release timeout window
                    None => {},
                }
            }

            State::Cooldown {
                key, release_time, click_count
            } => {
                // Calculate the time elapsed since the key was physically released
                let elapsed = now.saturating_sub(*release_time);

                match current_key {
                    // 1. combo-click detection, Same key pressed within the valid time window.
                    Some(curr) if curr == *key && elapsed < self.click_window => {
                        let count = *click_count + 1;
                        // The status must be reset to end the borrowing.
                        self.state = State::Idle;
                        self.enter_active(curr, now, count);
                    },
                    // 2. different key pressed or window expired, cooldown -> idle
                    Some(curr) => {
                        self.state = State::Idle;
                        return self.update(Some(curr));
                    },
                    // 3. timeout, clear cooldown state if no input is received within the window.
                    None if elapsed > self.click_window => {
                        self.state = State::Idle;
                    },
                    // 4. still in the cooldown window and no input
                    _ => {},
                }
            },
        }

        // return the next event produced by the logic above, if any.
        self.event_queue.pop_front()
    }

    /// Transition the state machine into the `Active` state and record the initial press.
    ///
    /// This helper function:
    /// 1. Updates the internal state to `Active` with the provided key and timestamp.
    /// 2. Resets the `long_press_triggered` flag for the new press cycle.
    /// 3. Sets the `click_count` (carried over from `Cooldown` or initialized to 1).
    /// 4. Pushes a `Pressed` event into the notification queue.
    ///
    /// #### Parameters
    /// * `key` - The metadata of the key being pressed.
    /// * `now` - The current hardware timestamp.
    /// * `count` - The current sequence count (e.g., 2 for a double-click).
    #[inline]
    fn enter_active(&mut self, key: KeyData, now: u64, count: usize) {
        self.state = State::Active {
            key,
            start_time: now,
            last_seen_time: now,
            long_press_triggered: false,
            click_count: count,
        };
        self.event_queue.push_back(InputEvent::Pressed(key));
    }

    /// Generates finalization events when a key is released or timed out.
    ///
    /// This helper handles the logic for ending a press cycle:
    /// 1. Always generates a `Released` event to signal the end of physical input.
    /// 2. Generates a `Click` event only if the press did not mature into a `LongPressed` event.
    ///
    /// By separating `Released` from `Click`, the system allows UI elements to clean up
    /// "held" states even if no logical click was registered.
    ///
    /// #### Parameters
    /// * `key` - The metadata of the key being released.
    /// * `lp_triggered` - Whether a `LongPressed` event was already sent for this cycle.
    /// * `count` - The final click count for this interaction.
    #[inline]
    fn emit_release_and_click(&mut self, key: &KeyData, lp_triggered: bool, count: usize) {
        self.event_queue.push_back(InputEvent::Released(*key));
        if !lp_triggered {
            self.event_queue.push_back(InputEvent::Click(*key, count));
        }
    }
}
