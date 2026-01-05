// Copyright (c) Bemly, January 2026
// You may copy and distribute this file freely.  Any queries and
// complaints should be forwarded to bemly_@petalmail.com.
// If you make any changes to this file, please do not distribute
// the results under the name `bemly'.

use core::sync::atomic::AtomicU64;

macro_rules! config {
    ($(
        $(#[$meta:meta])* $name:ident : $default:expr ;
    )*) => {
        $(
            static $name: AtomicU64 = AtomicU64::new($default);

            $(#[$meta])*
            #[allow(non_snake_case)]
            pub mod $name {
                use super::*;
                use core::sync::atomic::Ordering;

                /// Gets the current value.
                pub fn get() -> u64 {
                    $name.load(Ordering::Relaxed)
                }

                /// Sets a new value.
                pub fn set(t: u64) {
                    $name.store(t, Ordering::Relaxed)
                }
            }
        )*
    };
}

config! {
    /// Set timer to trigger every
    /// UEFI Spec V2.8 SetTimer => step is 100ns (unit)
    /// You can adjust this interval based on how "hot" you want the plug detection to be.
    ///
    /// Typical value: 10s (100000000)
    REFRESH_POSITIVE_INPUT_DEVICE_TIME: 1_0000_0000;

    /// The threshold duration used to determine if a key has been physically released.
    /// Since UEFI might not provide an explicit "release" interrupt, the state machine
    /// considers a key released if no signals are received within this window.
    ///
    /// Typical value: 150ms - 200ms.
    RELEASE_TIMEOUT: 150;

    /// The required duration of a continuous press before a `LongPressed` event is triggered.
    /// After this threshold, the state machine will begin emitting `Repeat` events.
    ///
    /// Typical value: 500ms.
    LONG_PRESS_DELAY: 500;

    /// The maximum time window between a key release and the next press to be
    /// considered a multi-click (e.g., a double click).
    ///
    /// Typical value: 250ms - 300ms.
    CLICK_WINDOW: 300;
}
