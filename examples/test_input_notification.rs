#![no_main]
#![no_std]

use core::hint::spin_loop;
use uefi::prelude::*;
use uefi::{print, println, Char16};
use uefi::proto::console::text::{Key, ScanCode};
use uefi_input2::simple_text_input_ex::{
    RawKeyData, KeyState, TOGGLE_STATE_VALID, CAPS_LOCK_ACTIVE, SHIFT_STATE_VALID
};
use uefi_input2::input::KeyData;

// Callback function for key notification
extern "efiapi" fn on_key_notify(_key_data: *mut RawKeyData) -> Status {
    print!("[listener]Key pressed: {:?}\n", _key_data);

    Status::SUCCESS
}

#[entry]
fn main() -> Status {
    uefi::helpers::init().expect("Failed to init UEFI");
    println!("UEFI Input Ex Integration Test");

    uefi_input2::with_stdin(|input| {
        println!("Testing on_key_notify listener...");

        let trigger_key = KeyData {
            key: Key::Printable(Char16::try_from('B').unwrap()),
            key_state: KeyState::default(),
        };
        match input.on_key_notify(&trigger_key, on_key_notify) {
            Ok(_handle) => {
                println!("   Notification registered for 'B'.");
            }
            Err(e) => {
                println!("   Failed to register notification: {:?}", e);
            }
        }

        loop {
            spin_loop();
        }

        Ok(())
    }).expect("Failed to access stdin");

    Status::SUCCESS
}
