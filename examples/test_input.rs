#![no_main]
#![no_std]
extern crate alloc;

use core::hint::spin_loop;
use uefi::prelude::*;
use uefi::{print, println};
use uefi::proto::console::text::Key::{Printable, Special};
use uefi::proto::console::text::ScanCode;

#[entry]
fn main() -> Status {
    uefi::helpers::init().expect("Failed to init UEFI");

    println!("Non-blocking SimpleTextInputEx Example");
    println!("Press keys to see them. Hold Ctrl/Shift/Alt to see modifiers.");
    println!("Press ESC to exit.");

    uefi_input2::with_stdin(|input| {
        loop {
            if let Some(key_data) = input.read_key_stroke_ex() {
                if key_data.is_realtime() {
                    print!("X");
                }
                if key_data.function_enable() {
                    if key_data.ctrl() {
                        print!("[Ctrl] ");
                    }
                    if key_data.shift() {
                        print!("[Shift] ");
                    }
                    if key_data.alt() {
                        print!("[Alt] ");
                    }
                }

                match key_data.key {
                    Printable(c) if u16::from(c) == 0x0D => print!("\r\n"),
                    // Only characters with a Unicode value
                    // greater than or equal to space (32) are truly printable.
                    Printable(c) if u16::from(c) >= 0x20 => print!("{}", c),
                    Special(code) if code == ScanCode::UP => print!("[Up] "),
                    Special(code) if code == ScanCode::DOWN => print!("[Down] "),
                    Special(code) if code == ScanCode::LEFT => print!("[Left] "),
                    Special(code) if code == ScanCode::RIGHT => print!("[Right] "),
                    Special(code) if code == ScanCode::ESCAPE => {
                        println!("Exiting...");
                        return Ok(())
                    },
                    // Special(code) => print!("{}", format!("{:?}", code).as_str()),
                    _ => {}
                }
            }

            spin_loop()
        }
    }).expect("Failed to get stdin");

    Status::SUCCESS
}
