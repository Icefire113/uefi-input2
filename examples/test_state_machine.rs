#![no_main]
#![no_std]

use core::hint::spin_loop;
use uefi::prelude::*;
use uefi::{print, println};
use uefi::proto::console::text::Key::{Printable, Special};
use uefi::proto::console::text::ScanCode;
use uefi_input2::key_data::KeyData;
use uefi_input2::state_machine::{InputEvent, StateMachine};
use uefi_input2::with_stdins;

#[entry]
fn main() -> Status {
    uefi::helpers::init().expect("Failed to init UEFI");

    // 初始化状态机
    let mut sm = StateMachine::new().expect("状态机初始化失败");

    with_stdins(|input| {
        for keyboard in input.iter_mut() {
            let _ = KeyData::realtime_init(keyboard, true);
        }
        
        loop {
            spin_loop();

            for keyboard in input.iter_mut() {
                // A. 非阻塞读取当前按键
                // read_key_stroke_ex 是非阻塞的，如果没有按键，它会返回 NOT_READY
                let uefi_key = keyboard.read_key_stroke_ex();

                // C. 更新状态机
                // 即使 current_key_data 是 None，也必须调用 update！
                // 因为状态机需要检测“按键释放超时”和“双击窗口超时”。
                if let Some(event) = sm.update(uefi_key) {
                    match event {
                        InputEvent::LongPressed(k) => {
                            match k.key {
                                Printable(c) if u16::from(c) == 0x0D => print!("\r\n"),
                                Printable(c) if u16::from(c) >= 0x20 => print!("[long {}]", c),
                                Special(c) => print!("[long {:?}]", c),
                                _ => {},
                            }
                        }
                        InputEvent::Repeat(k) => {
                            match k.key {
                                Printable(c) if u16::from(c) == 0x0D => print!("\r\n"),
                                Printable(c) if u16::from(c) >= 0x20 => print!("[repeat {}]", c),
                                Special(c) => print!("[repeat {:?}]", c),
                                _ => {},
                            }
                        }
                        InputEvent::Pressed(k) => {
                            match k.key {
                                Printable(c) if u16::from(c) == 0x0D => print!("\r\n"),
                                Printable(c) if u16::from(c) >= 0x20 => print!("[pressed {}]", c),
                                Special(c) => print!("[pressed {:?}]", c),
                                _ => {},
                            }
                        }
                        InputEvent::Released(k) => {
                            match k.key {
                                Printable(c) if u16::from(c) == 0x0D => print!("\r\n"),
                                Printable(c) if u16::from(c) >= 0x20 => print!("[released {}]", c),
                                Special(c) => print!("[released {:?}]", c),
                                _ => {},
                            }
                        },
                        InputEvent::Click(k, count) => {
                            match k.key {
                                Printable(c) if u16::from(c) == 0x0D => print!("\r\n"),
                                Printable(c) if u16::from(c) >= 0x20 => print!("[click {} {}]", c, count),
                                Special(c) => print!("[click {:?} {}]", c, count),
                                _ => {},
                            }
                        },
                    }
                }
            }
        }

        Ok(())
    }).expect("无法打开 stdin");

    Status::SUCCESS
}