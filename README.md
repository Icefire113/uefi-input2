UEFI Simple Text Input Ex Protocol Wrapper
==============================================
[![CI](https://github.com/Bemly/uefi-input2/workflows/CI/badge.svg)](https://github.com/Bemly/uefi-input2/actions/workflows/ci.yml)
[![Docs.rs](https://docs.rs/uefi-input2/badge.svg)](https://docs.rs/uefi-input2)
[![Crates.io](https://img.shields.io/crates/v/uefi-input2.svg)](https://crates.io/crates/uefi-input2)
[![License: Zed](https://img.shields.io/badge/License-Zed-yellow.svg)](https://spdx.org/licenses/Zed.html)
[![Rust](https://img.shields.io/badge/rust-2024%20edtion-blue.svg)](https://www.rust-lang.org)
[![UEFI](https://img.shields.io/badge/target-*--unknown--uefi-orange.svg)](https://doc.rust-lang.org/nightly/rustc/platform-support/unknown-uefi.html)

This library provides a safe, idiomatic Rust wrapper for the `EFI_SIMPLE_TEXT_INPUT_EX_PROTOCOL`.
Unlike the standard `SimpleTextInput`, this protocol allows for advanced key tracking,
including shift state (Ctrl, Alt, Shift) and toggle state (Caps Lock, Num Lock).

Features
----------------------------------------------
- **Seamless Migration**: Designed as a **drop-in, painless replacement**
  for the standard `uefi::system::with_stdin`.
- **Safe Resource Management**: Uses the `with_stdin` pattern to ensure the protocol is opened
  exclusively and closed automatically.
- **Extended Key Data**: Access to `KeyShiftState` and `KeyToggleState`.
- **No-Std Compatible**: Designed specifically for UEFI environments.

Usage
----------------------------------------------
Simply replace your import and use the same closure-based pattern:

```rust,no_run
#![no_main]
#![no_std]
use uefi::prelude::*;
use uefi::{print, println};
use uefi::proto::console::text::Key::{Printable, Special};
use uefi::proto::console::text::ScanCode;
use uefi_input2::with_stdin;
use uefi_input2::simple_text_input_ex::{LEFT_SHIFT_PRESSED, RIGHT_SHIFT_PRESSED};

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    with_stdin(|input| {
        loop {
            if let Some(data) = input.read_key_stroke_ex() {
                if data.shift() { println!("Shift is being held!") }
                match data.key {
                   Printable(c) if u16::from(c) == 0x0D => print!("\r\n"),
                   Printable(c) => print!("{}", c),
                   Special(code) if code == ScanCode::ESCAPE => {
                       println!("Exiting...");
                       return Ok(())
                   },
                   _ => {}
               }
            }
        }
        Ok(())
    }).unwrap();

    Status::SUCCESS
}
```

Test
----------------------------------------------
Due to a reverse-execution bug in the RustRover README runner,
this script is intentionally authored in reverse order for compatibility.
```shell
qemu-system-x86_64 -drive if=pflash,format=raw,file=qemu/OVMF.fd -drive format=raw,file=fat:rw:qemu -m 4G -device usb-ehci -device usb-tablet -smp 4 -cpu max -monitor stdio
mv -Force .\target\x86_64-unknown-uefi\debug\examples\*.efi .\qemu\EFI\BOOT\BOOTX64.EFI
rm .\qemu\EFI\BOOT\BOOTX64.EFI
cargo build --example test_input
```

License
----------------------------------------------
This project is licensed under the Zed License.