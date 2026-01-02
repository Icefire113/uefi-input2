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

    println!("Non-blocking SimpleTextInputEx Example");
    println!("Press keys to see them. Hold Ctrl/Shift/Alt to see modifiers.");
    println!("Press ESC to exit.");

    uefi_input2::with_stdin(|input| {
        loop {
            if let Some(key_data) = input.read_key_stroke_ex() {
                let key = key_data.key;
                let shift_state = key_data.key_state.key_shift_state;

                if shift_state & SHIFT_STATE_VALID != 0 {
                    if shift_state & (LEFT_CONTROL_PRESSED | RIGHT_CONTROL_PRESSED) != 0 {
                        print!("[Ctrl] ");
                    }
                    if shift_state & (LEFT_SHIFT_PRESSED | RIGHT_SHIFT_PRESSED) != 0 {
                        print!("[Shift] ");
                    }
                    if shift_state & (LEFT_ALT_PRESSED | RIGHT_ALT_PRESSED) != 0 {
                        print!("[Alt] ");
                    }
                }

                match key {
                    Printable(c) if u16::from(c) == 0x0D => print!("\r\n"),
                    Printable(c) => print!("{}", c),
                    Special(code) if code == ScanCode::UP => print!("[Up] "),
                    Special(code) if code == ScanCode::DOWN => print!("[Down] "),
                    Special(code) if code == ScanCode::LEFT => print!("[Left] "),
                    Special(code) if code == ScanCode::RIGHT => print!("[Right] "),
                    Special(code) if code == ScanCode::ESCAPE => {
                        println!("Exiting...");
                        return Ok(())
                    },
                    _ => {}
                }
            }

            spin_loop()
        }
    }).expect("Failed to get stdin");

    Status::SUCCESS
}
