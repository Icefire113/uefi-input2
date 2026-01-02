

Test
----------
Please run the following script in reverse order, because the RustRover README executable has a bug and will execute backwards.
The author wrote it in reverse order for convenience.
```shell
qemu-system-x86_64 -drive if=pflash,format=raw,file=qemu/OVMF.fd -drive format=raw,file=fat:rw:qemu -m 4G -device usb-ehci -device usb-tablet -smp 4 -cpu max -monitor stdio
mv -Force .\target\x86_64-unknown-uefi\debug\examples\test_inpu*.efi .\qemu\EFI\BOOT\BOOTX64.EFI
rm .\qemu\EFI\BOOT\BOOTX64.EFI
cargo build --example test_input
```