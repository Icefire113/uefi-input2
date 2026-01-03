#![no_main]
#![no_std]

use core::hint::spin_loop;
use uefi::prelude::*;
use uefi::println;
use uefi::proto::console::text::Key::Special;
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
                        InputEvent::SingleClick(k) => {
                            println!("click {:?}", k);
                        }
                        InputEvent::DoubleClick(k) => {
                            println!("double click {:?}", k);
                        }
                        InputEvent::LongPressed(k) => {
                            println!("long press {:?}", k);
                        }
                        InputEvent::Repeat(k) => {
                            println!("multi click {:?}", k);
                        }
                        InputEvent::Pressed(k) => {
                            if let Special(c) = k.key {
                                if c == ScanCode::ESCAPE {
                                    break;
                                }
                            }
                        }
                        InputEvent::Released(k) => {
                            println!("Released {:?}", k);
                        }
                    }
                }
            }
        }

        Ok(())
    }).expect("无法打开 stdin");

    Status::SUCCESS
}