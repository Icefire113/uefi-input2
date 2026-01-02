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
        // 1. Test Reset
        println!("1. Testing reset(true)...");
        if let Err(e) = input.reset(true) {
            println!("   Reset failed: {:?}", e);
        } else {
            println!("   Reset successful.");
        }

        // 2. Test Set State (Turn on CapsLock)
        println!("2. Testing set_state (CapsLock)...");
        let caps_state = TOGGLE_STATE_VALID | CAPS_LOCK_ACTIVE;
        if let Err(e) = input.set_state(caps_state) {
            println!("   Set state failed: {:?}", e);
        } else {
            println!("   Set state successful (CapsLock should be on).");
        }

        // 3. Test on_key_notify
        println!("3. Testing on_key_notify drop...");
        let trigger_key = KeyData {
            key: Key::Printable(Char16::try_from('c').unwrap()),
            key_state: KeyState::default(),
        };

        println!("4. notification automatic dropped.");
        {
            // 4. Register notification in a scope
            match input.on_key_callback(&trigger_key, on_key_notify) {
                Ok(_handle) => {
                    println!("   Notification registered for 'c'.");
                    println!("   (Notification handle will be dropped now to allow input loop)");
                }
                Err(e) => {
                    println!("   Failed to register notification: {:?}", e);
                }
            }
            // _handle drops here, unregistering the notification
        }

        println!("5. Listener key event. Press Any to continue.");

        {
            // 5. Test wait_for_key_event(blocking)
            let event = input.wait_for_key_event().expect("Failed to get wait event");
            let mut events = [event];

            // Wait for event
            if boot::wait_for_event(&mut events).is_err() {
                println!("   Wait for event failed.");
            }
        }

        println!("6. Entering input loop. Press 'ESC' to exit.");

        loop {
            // 6. Test wait_for_key_event(non-blocking)
            let event = input.wait_for_key_event().expect("Failed to get wait event");
            let mut events = [event];

            // Wait for event
            if boot::wait_for_event(&mut events).is_err() {
                println!("   Wait for event failed.");
                return Ok(());
            }
            // Read key
            if let Some(key_data) = input.read_key_stroke_ex() {
                print!("   Key pressed: {:?}", key_data.key);

                if key_data.key_state.key_shift_state & SHIFT_STATE_VALID != 0 {
                    print!(" | ShiftState: 0x{:08X}", key_data.key_state.key_shift_state);
                }
                if key_data.key_state.key_toggle_state & TOGGLE_STATE_VALID != 0 {
                    print!(" | ToggleState: 0x{:02X}", key_data.key_state.key_toggle_state);
                }
                println!();

                if let Key::Special(ScanCode::ESCAPE) = key_data.key {
                    return Ok(());
                }
            }
            spin_loop();
        }
    }).expect("Failed to access stdin");

    Status::SUCCESS
}
