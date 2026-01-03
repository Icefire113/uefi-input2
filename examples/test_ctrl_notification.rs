#![no_main]
#![no_std]

use core::hint::spin_loop;
use uefi::prelude::*;
use uefi::{println, Char16};
use uefi::proto::console::text::Key;
use uefi_input2::simple_text_input_ex::{RawKeyData, KeyState, KEY_STATE_EXPOSED, LEFT_CONTROL_PRESSED, TOGGLE_STATE_VALID, SHIFT_STATE_VALID};
use uefi_input2::key_data::KeyData;

extern "efiapi" fn on_key_notify(key_data: *mut RawKeyData) -> Status {
    if key_data.is_null() { return Status::INVALID_PARAMETER }
    let key_data = unsafe { &*key_data };
    println!("Key pressed: {:?}", key_data);

    Status::SUCCESS
}

#[allow(unreachable_code)]
#[entry]
fn main() -> Status {

    println!("Ctrl Notice Integration Test");

    uefi_input2::with_stdin(|input| {
        KeyData::realtime_init(input, true)?;

        // check if left ctrl pressed
        let trigger_key = KeyData {
            key: Key::Printable(Char16::try_from('\0').unwrap()),
            key_state: KeyState {
                key_toggle_state: KEY_STATE_EXPOSED | TOGGLE_STATE_VALID,
                key_shift_state: LEFT_CONTROL_PRESSED | SHIFT_STATE_VALID,
            },
        };

        // receive handle outside of match
        let _handle = match input.on_key_callback(&trigger_key, on_key_notify) {
            Ok(h) => {
                println!("Notification registered.");
                h // Return the handle to _handle
            }
            Err(e) => {
                println!("Failed to register notification: {:?}", e);
                return Err(e);
            }
        };

        loop {

            spin_loop();
        }

        Ok(())
    }).expect("Failed to access stdin");

    Status::SUCCESS
}
