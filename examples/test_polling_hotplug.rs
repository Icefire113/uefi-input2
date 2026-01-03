// #![no_main]
// #![no_std]
// extern crate alloc;
// 
// use alloc::vec::Vec;
// use core::hint::spin_loop;
// use core::ptr::NonNull;
// use core::time::Duration;
// use uefi::prelude::*;
// use uefi::{print, println};
// use uefi::boot::{get_handle_for_protocol, OpenProtocolAttributes, OpenProtocolParams, ScopedProtocol, SearchType};
// use uefi_input2::input::Input;
// use uefi_input2::KeyboardHotPlugMonitor;
// 
// pub fn refresh_keyboard_list(existing_keyboards: &mut Vec<ScopedProtocol<Input>>) {
//     // 1. 暴力清理：不管三七二十一，先把所有的 ScopedProtocol 丢弃
//     // 这样能确保所有的 Exclusive 独占权全部归还给系统
//     existing_keyboards.clear();
// 
//     // 2. 重新扫描全场
//     let Ok(handles) = boot::locate_handle_buffer(SearchType::AllHandles) else { return; };
// 
//     for handle in handles.iter() {
//         // 3. 尝试打开
//         // 注意：有些句柄是系统的 ConIn，有些是物理 USB，全部尝试一遍
//         match boot::open_protocol_exclusive::<Input>(*handle) {
//             Ok(keyboard) => {
//                 existing_keyboards.push(keyboard);
//                 println!("Found keyboard");
//             },
//             Err(status) => {
//             }
//         }
//     }
// }
// 
// #[entry]
// fn main() -> Status {
//     let mut keyboard_dirty = false;
//     let mut keyboards: Vec<ScopedProtocol<Input>> = Vec::new();
// 
//     // 初始化时先扫一遍现有的键盘
//     refresh_keyboard_list(&mut keyboards);
// 
//     // 启动热插拔监听
//     let flag_ptr = NonNull::new(&mut keyboard_dirty as *mut bool).unwrap();
//     let _monitor = KeyboardHotPlugMonitor::new(flag_ptr).expect("Failed to start monitor");
// 
//     let mut loop_count: u32 = 0;
//     // 设定刷新频率：例如每 500 次循环（配合底层的 stall，大约 1-2 秒）强制检查一次
//     const REFRESH_THRESHOLD: u32 = 300000;
// 
//     loop {
//         // --- 方案 A：每隔一段时间强制刷一遍（暴力但最稳） ---
//         // 或者方案 B：检测到读取错误时触发刷新
// 
//         let mut needs_refresh = keyboard_dirty; // 信号触发了肯定要刷
// 
//         // 关键：主动检查现有的键盘是否还有效
//         // 如果发现列表里有键盘“断气”了，即使没收到信号，也要强制刷新
//         if !needs_refresh {
//             loop_count += 1;
//             if loop_count >= REFRESH_THRESHOLD {
//                 needs_refresh = true;
//                 loop_count = 0;
//             }
// 
//             for k in keyboards.iter_mut() {
//                 // 使用 reset(false) 作为心跳检测
//                 // 如果键盘拔了，这里会立即返回 Err
//                 if k.reset(false).is_err() {
//                     needs_refresh = true;
//                     println!("检测到键盘断开，准备重置列表...");
//                     break;
//                 }
//             }
//         }
// 
//         if needs_refresh {
//             keyboard_dirty = false;
// 
//             // 1. 必须 clear：这会触发 CloseProtocol，告诉固件“我放手了”
//             keyboards.clear();
// 
//             // 2. 适当延时：给固件一点时间处理硬件状态变更（可选）
// 
//             // 3. 全量重新识别
//             refresh_keyboard_list(&mut keyboards);
//         }
// 
//         // --- 正常的业务逻辑 ---
//         for k in keyboards.iter_mut() {
//             if let Some(key) = k.read_key_stroke_ex() {
//                 // ... 处理按键 ...
//                 println!("{:?}", key.key);
//             }
//         }
//     }
// }
