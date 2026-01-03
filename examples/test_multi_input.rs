#![no_main]
#![no_std]
use uefi::prelude::*;
use uefi::{print, println, Error, Result};
use uefi::boot::ScopedProtocol;
use uefi::proto::console::text::Key::{Printable, Special};
use uefi::proto::console::text::ScanCode;
use uefi_input2::input::Input;
use core::hint::spin_loop;

#[allow(unreachable_code)]
#[entry]
fn main() -> Status {
    uefi::helpers::init().expect("Failed to init UEFI");

    println!("Multi Keyboard Input Integration Test");


    uefi_input2::with_stdins(|stdins| {
        loop {
            spin_loop();

            #[cfg(feature = "alloc")]
            for keyboard in stdins.iter_mut() {
                if test_multi_input(keyboard).is_err() { return Ok(()) }
            }

            #[cfg(not(feature = "alloc"))]
            for keyboard in stdins.iter_mut().flatten() {
                if test_multi_input(keyboard).is_err() { return Ok(()) }
            }
        }

        Ok(())
    }).expect("Failed to get stdins");


    Status::SUCCESS
}

#[inline]
fn test_multi_input(keyboard: &mut ScopedProtocol<Input>) -> Result {
    if let Some(key_data) = keyboard.read_key_stroke_ex() {
        if key_data.is_realtime_mode() {
            print!("X");
        }
        if key_data.supports_modifiers() && key_data.ctrl() {
            print!("[Ctrl] ");
        }
        match key_data.key {
            Printable(c) if u16::from(c) == 0x0D => Ok(print!("\r\n")),
            Printable(c) if u16::from(c) >= 0x20 => Ok(print!("{}", c)),
            Special(code) if code == ScanCode::ESCAPE => {
                println!("Exiting...");
                Err(Error::from(Status::SUCCESS))
            },
            _ => Ok(()),
        }
    } else {
        Ok(())
    }
}
