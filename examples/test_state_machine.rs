#![no_main]
#![no_std]

use core::hint::spin_loop;
use uefi::prelude::*;
use uefi::print;
use uefi::proto::console::text::Key::{Printable, Special};
use uefi_input2::key_data::KeyData;
use uefi_input2::state_machine::{InputEvent, StateMachine};
use uefi_input2::{config, with_stdins};

#[allow(unreachable_code)]
#[entry]
fn main() -> Status {
    uefi::helpers::init().expect("Failed to init UEFI");
    
    // The delay can be modified before initializing the state machine.
    config::RELEASE_TIMEOUT::set(200);
    // Initialize the state machine
    let mut sm = StateMachine::new().expect("init failed.");

    with_stdins(|input| {
        for keyboard in input.iter_mut() {
            let _ = KeyData::realtime_init(keyboard, true);
        }

        loop {
            spin_loop();

            for keyboard in input.iter_mut() {
                // non-blocking read of current key press
                // `read_key_stroke_ex` is non-blocking; if no key is pressed,
                // it will return `NOT_READY`.
                let uefi_key = keyboard.read_key_stroke_ex();

                // update must be called even if current_key_data is None.
                // because the state machine needs to check for
                // "key release timeout" and "double-click window timeout".
                if let Some(event) = sm.update(uefi_key) {
                    // classification
                    let (label, k, count) = match event {
                        InputEvent::Pressed(k)        => ("pressed", k, None),
                        InputEvent::Released(k)       => ("released", k, None),
                        InputEvent::LongPressed(k)    => ("long", k, None),
                        InputEvent::Repeat(k)         => ("repeat", k, None),
                        InputEvent::Click(k, c) => ("click", k, Some(c)),
                    };

                    // print
                    match k.key {
                        Printable(c) if u16::from(c) == 0x0D => print!("\r\n"),
                        Printable(c) if u16::from(c) >= 0x20 => {
                            if let Some(c_val) = count {
                                print!("[{} {} {}]", label, c, c_val)
                            } else {
                                print!("[{} {}]", label, c)
                            }
                        },
                        Special(c) => {
                            if let Some(c_val) = count {
                                print!("[{} {:?} {}]", label, c, c_val)
                            } else {
                                print!("[{} {:?}]", label, c)
                            }
                        },
                        _ => {},
                    }
                }
            }
        }

        Ok(())
    }).expect("stdin open failed.");

    Status::SUCCESS
}