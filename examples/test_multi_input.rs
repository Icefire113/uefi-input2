#![no_main]
#![no_std]
use uefi::prelude::*;
use uefi::println;

#[entry]
fn main() -> Status {
    uefi::helpers::init().expect("Failed to init UEFI");

    println!("Multi Keyboard Input Integration Test");

    #[cfg(feature = "alloc")]
    uefi_input2::with_stdins(|stdins| {
        use uefi::proto::console::text::Key::{Printable, Special};
        use uefi::proto::console::text::ScanCode;
        use core::hint::spin_loop;
        use uefi::print;

        loop {
            spin_loop();

            for keyboard in stdins.iter_mut() {
                if let Some(key_data) = keyboard.read_key_stroke_ex() {
                    if key_data.is_realtime() {
                        print!("X");
                    }
                    if key_data.function_enable() && key_data.ctrl() {
                        print!("[Ctrl] ");
                    }
                    match key_data.key {
                        Printable(c) if u16::from(c) == 0x0D => print!("\r\n"),
                        Printable(c) if u16::from(c) >= 0x20 => print!("{}", c),
                        Special(code) if code == ScanCode::ESCAPE => {
                            println!("Exiting...");
                            return Ok(());
                        },
                        _ => {}
                    }
                }
            }
        }
    }).expect("Failed to get stdins");


    Status::SUCCESS
}
