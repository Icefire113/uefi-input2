#![no_main]
#![no_std]

use core::hint::spin_loop;
use uefi::prelude::*;
use uefi::{print, println, Char16};
use uefi::proto::console::text::Key;
use uefi_input2::simple_text_input_ex::{
    RawKeyData, KeyState
};
use uefi_input2::input::KeyData;

// Callback function for key notification
extern "efiapi" fn on_key_notify(_key_data: *mut RawKeyData) -> Status {
    print!("[listener]Key pressed: {:?}\n", _key_data);

    Status::SUCCESS
}

#[allow(unreachable_code)]
#[entry]
fn main() -> Status {
    uefi::helpers::init().expect("Failed to init UEFI");
    println!("UEFI Input Ex Integration Test");

    uefi_input2::with_stdin(|input| {
        let trigger_key = KeyData {
            key: Key::Printable(Char16::try_from('b').unwrap()),
            key_state: KeyState::default(),
        };

        // receive handle outside of match
        let _handle = match input.on_key_callback(&trigger_key, on_key_notify) {
            Ok(h) => {
                println!("   Notification registered for 'b'.");
                h // Return the handle to _handle
            }
            Err(e) => {
                println!("   Failed to register notification: {:?}", e);
                return Err(e); // exit the closure if registration fails.
            }
        };

        // _handle is still alive until the closure of with_stdin ends.
        println!("   Waiting for 'b' key... (Notification is ACTIVE)");
        loop {
            spin_loop();
        }

        Ok(())
    }).expect("Failed to access stdin");

    Status::SUCCESS
}
