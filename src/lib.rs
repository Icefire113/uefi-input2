// Copyright (c) Bemly, January 2026
// You may copy and distribute this file freely.  Any queries and
// complaints should be forwarded to bemly_@petalmail.com.
// If you make any changes to this file, please do not distribute
// the results under the name `bemly'.

//! # UEFI Simple Text Input Ex Protocol Wrapper
//!
//! This library provides a safe, idiomatic Rust wrapper for the `EFI_SIMPLE_TEXT_INPUT_EX_PROTOCOL`.
//! Unlike the standard `SimpleTextInput`, this protocol allows for advanced key tracking,
//! including shift state (Ctrl, Alt, Shift) and toggle state (Caps Lock, Num Lock).
//!
//! ## Purpose
//! - **Seamless Migration**: Designed as a **drop-in, painless replacement**
//!     for the standard `uefi::system::with_stdin`.
//! - **Safe Resource Management**: Uses the `with_stdin` pattern to ensure the protocol is opened
//!     exclusively and closed automatically.
//! - **Extended Key Data**: Access to `KeyShiftState` and `KeyToggleState`.
//! - **No-Std Compatible**: Designed specifically for UEFI environments.
//!
//! ## Feature List
//! - **alloc**: Enables `Vec` support. For example, `with_stdins` requires
//!     the `alloc` feature to more than 8 multiple input handles via `find_handles`.
//!
//! ## Minimal Example
//! Simply replace your import and use the same closure-based pattern:
//!
//! ```rust,no_run
//! #![no_main]
//! #![no_std]
//! use uefi::prelude::*;
//! use uefi::{print, println};
//! use uefi::proto::console::text::Key::{Printable, Special};
//! use uefi::proto::console::text::ScanCode;
//! use uefi::boot::check_event;
//!
//! #[entry]
//! fn main() -> Status {
//!     uefi::helpers::init().unwrap();
//!
//!     uefi_input2::with_stdin(|input| {
//!         loop {
//!             // Performance Note: Using wait_for_key_event + check_event conforms to UEFI
//!             // best practices by reducing CPU overhead and bus traffic. However,
//!             // for maximum loop throughput (e.g., high-frequency GOP rendering),
//!             // consider calling read_key_stroke_ex directly to save the extra protocol call.
//!             let Some(event) = input.wait_for_key_event() else { continue };
//!             if !check_event(event)? { continue }
//!
//!             if let Some(data) = input.read_key_stroke_ex() {
//!                 if data.shift() { println!("Shift is being held!") }
//!                 match data.key {
//!                    Printable(c) if u16::from(c) == 0x0D => print!("\r\n"),
//!                    Printable(c) => print!("{}", c),
//!                    Special(code) if code == ScanCode::ESCAPE => {
//!                        println!("Exiting...");
//!                        return Ok(())
//!                    },
//!                    _ => {},
//!                }
//!             }
//!         }
//!         Ok(())
//!     }).unwrap();
//!
//!     Status::SUCCESS
//! }
//! ```

#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]

/// 
pub mod config;
/// C FFI Bindingw
pub mod simple_text_input_ex;
/// simple_text_input_ex wrapper
pub mod input;
/// height-level data wrapper
pub mod key_data;
/// keyboard hotplug support (Not Recommend, Unplugging is UEFI Spec Undefined Behavior)
#[cfg(feature = "alloc")]
pub mod hotplug;
/// Core input state machine for handling complex keyboard events.
#[cfg(feature = "alloc")]
pub mod state_machine;
/// Compatibility layer for environments lacking UEFI `Timestamp` protocol support.
#[cfg(feature = "alloc")]
mod state_machine_fallback;

use uefi::boot::{get_handle_for_protocol, open_protocol_exclusive, ScopedProtocol};
use uefi::Result;
use crate::input::Input;


/// Reads the high-resolution hardware cycle counter for the current CPU architecture.
///
/// This function provides a low-overhead timestamp used for frequency calibration
/// and duration measurements. It abstracts over different CPU architectures:
///
/// - **x86 / x86_64**: Uses the `RDTSC` (Read Time Stamp Counter) instruction.
/// - **AArch64**: Reads the `CNTVCT_EL0` (Virtual Count Register) via the system register interface.
///
/// #### Safety
/// This function is marked as `unsafe` internally because it uses direct hardware
/// instructions and inline assembly.
///
/// - On x86, the TSC is not strictly guaranteed to be synchronized across multiple cores
///   or constant across frequency scaling (though it is on most modern "Constant TSC" CPUs).
/// - In the UEFI environment, which is typically single-threaded, these concerns are minimized.
///
/// #### Returns
/// A 64-bit unsigned integer representing the current hardware tick count.
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


/// it has roughly the same function as `uefi::system::with_stdin`.
/// only support single keyboard.
pub fn with_stdin<F, R>(mut f: F) -> Result<R>
where F: FnMut(&mut ScopedProtocol<Input>) -> Result<R> {
    let input = get_handle_for_protocol::<Input>()?;
    let mut input = open_protocol_exclusive::<Input>(input)?;

    f(&mut input)
}

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(feature = "alloc")]
use uefi::boot::find_handles;

/// support multiple keyboard.
///
/// Tips: if OEM UEFI impl ConSplitter driver(Virtual ConIn), keyboard hotplug may is supported.
///
/// #### Usage
/// ```rust,no_run
/// uefi_input2::with_stdins(|stdins| {
///     loop {
///         for keyboard in stdins.iter_mut() {
///             let Some(event) = input.wait_for_key_event() else { continue };
///             if !check_event(event)? { continue }
///
///             if let Some(key_data) = keyboard.read_key_stroke_ex() {
///                 // just do it!
///             }
///         }
///     }
///  }).unwarp();
/// ```
#[cfg(feature = "alloc")]
pub fn with_stdins<F, R>(mut f: F) -> Result<R>
where F: FnMut(&mut Vec<ScopedProtocol<Input>>) -> Result<R> {

    let inputs = find_handles::<Input>()?;
    let mut keyboards: Vec<ScopedProtocol<Input>> = Vec::with_capacity(inputs.len());
    for input in inputs {
        let keyboard = open_protocol_exclusive::<Input>(input)?;
        keyboards.push(keyboard);
    }

    f(&mut keyboards)
}

/// only supports a maximum of 8 keyboards.
#[cfg(not(feature = "alloc"))]
pub fn with_stdins<F, R>(mut f: F) -> Result<R>
where F: FnMut(&mut [Option<ScopedProtocol<Input>>]) -> Result<R> {
    use uefi::boot::{locate_handle_buffer, SearchType};
    use uefi::Identify;

    let inputs = locate_handle_buffer(SearchType::ByProtocol(&Input::GUID))?;

    let mut keyboards: [Option<ScopedProtocol<Input>>; 8]
        =  [None, None, None, None, None, None, None, None];

    for (i, &input) in inputs.iter().enumerate() {
        if i >= keyboards.len() { break } // safe check

        if let Ok(keyboard) = open_protocol_exclusive::<Input>(input) {
            keyboards[i] = Some(keyboard);
        }
    }
    f(&mut keyboards)
}