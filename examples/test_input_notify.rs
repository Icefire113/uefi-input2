#![no_main]
#![no_std]

use core::hint::spin_loop;
use uefi::prelude::*;
use uefi::{print, println};
use uefi::proto::console::text::Key::{Printable, Special};
use uefi::proto::console::text::ScanCode;
use uefi_input2::simple_text_input_ex::{LEFT_ALT_PRESSED, LEFT_CONTROL_PRESSED, LEFT_SHIFT_PRESSED, RIGHT_ALT_PRESSED, RIGHT_CONTROL_PRESSED, RIGHT_SHIFT_PRESSED, SHIFT_STATE_VALID};

#[entry]
fn main() -> Status {
    uefi::helpers::init().expect("Failed to init UEFI");

    uefi_input2::with_stdin(|input| {

        Ok(())
    }).expect("Failed to get stdin");

    Status::SUCCESS
}
