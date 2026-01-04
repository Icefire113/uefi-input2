use core::sync::atomic::{AtomicU64, Ordering};

///
static REFRESH_POSITIVE_INPUT_DEVICE_TIME: AtomicU64 = AtomicU64::new(1_0000_0000);

/// The threshold duration used to determine if a key has been physically released.
/// Since UEFI might not provide an explicit "release" interrupt, the state machine
/// considers a key released if no signals are received within this window.
///
/// Typical value: 150ms - 200ms.
static RELEASE_TIMEOUT: AtomicU64 = AtomicU64::new(150);

/// The required duration of a continuous press before a `LongPressed` event is triggered.
/// After this threshold, the state machine will begin emitting `Repeat` events.
///
/// Typical value: 500ms.
static LONG_PRESS_DELAY: AtomicU64 = AtomicU64::new(500);

/// The maximum time window between a key release and the next press to be
/// considered a multi-click (e.g., a double click).
///
/// Typical value: 250ms - 300ms.
static CLICK_WINDOW: AtomicU64 = AtomicU64::new(300);

pub fn refresh_positive_input_device_time() -> u64 {
    REFRESH_POSITIVE_INPUT_DEVICE_TIME.load(Ordering::Relaxed)
}
pub fn set_refresh_positive_input_device_time(ns100: u64) {
    REFRESH_POSITIVE_INPUT_DEVICE_TIME.store(ns100, Ordering::Relaxed)
}
pub fn release_timeout() -> u64 { RELEASE_TIMEOUT.load(Ordering::Relaxed) }
pub fn set_release_timeout(ms: u64) { RELEASE_TIMEOUT.store(ms, Ordering::Relaxed) }
pub fn long_press_delay() -> u64 { LONG_PRESS_DELAY.load(Ordering::Relaxed) }
pub fn set_long_press_delay(ms: u64) { LONG_PRESS_DELAY.store(ms, Ordering::Relaxed) }
pub fn click_window() -> u64 { CLICK_WINDOW.load(Ordering::Relaxed) }
pub fn set_click_window(ms: u64) { CLICK_WINDOW.store(ms, Ordering::Relaxed) }