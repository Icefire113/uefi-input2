//! # UEFI Simple Text Input Ex Protocol Wrapper
//!
//! This library provides a safe, idiomatic Rust wrapper for the `EFI_SIMPLE_TEXT_INPUT_EX_PROTOCOL`.
//! Unlike the standard `SimpleTextInput`, this protocol allows for advanced key tracking,
//! including shift state (Ctrl, Alt, Shift) and toggle state (Caps Lock, Num Lock).
//!
//! ## Features
//! - **Seamless Migration**: Designed as a **drop-in, painless replacement**
//!     for the standard `uefi::system::with_stdin`.
//! - **Safe Resource Management**: Uses the `with_stdin` pattern to ensure the protocol is opened
//!     exclusively and closed automatically.
//! - **Extended Key Data**: Access to `KeyShiftState` and `KeyToggleState`.
//! - **No-Std Compatible**: Designed specifically for UEFI environments.
//!
//! ## Usage
//! Simply replace your import and use the same closure-based pattern:
//! 
//! ```rust
//! use uefi_input2::with_stdin;
//! use uefi_input2::simple_text_input_ex::{LEFT_SHIFT_PRESSED, RIGHT_SHIFT_PRESSED};
//!
//! #[entry]
//! fn main() -> Status {
//!     uefi::helpers::init().unwrap();
//!
//!     with_stdin(|input| {
//!         loop {
//!             if let Some(data) = input.read_key_stroke_ex() {
//!                 // Check if any Shift key is held
//!                 let is_shift = data.key_state.key_shift_state & (LEFT_SHIFT_PRESSED | RIGHT_SHIFT_PRESSED) != 0;
//!                 
//!                 if is_shift {
//!                     uefi::println!("Shift is being held!");
//!                 }
//!
//!                 // Exit on ESC (Scan Code 0x17)
//!                 if data.key.scan_code == 0x17 { break; }
//!             }
//!         }
//!         Ok(())
//!     }).unwrap();
//!
//!     Status::SUCCESS
//! }
//! ```

#![no_std]

use uefi::boot::{get_handle_for_protocol, open_protocol_exclusive};
use uefi::Result;
use crate::input::Input;

pub mod simple_text_input_ex;
pub mod input;

pub fn with_stdin<F, R>(mut f: F) -> uefi::Result<R>
where
    F: FnMut(&mut Input) -> Result<R>
{
    let input = get_handle_for_protocol::<Input>()?;
    let mut input = open_protocol_exclusive::<Input>(input)?;

    f(&mut *input)
}