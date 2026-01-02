use core::ptr::addr_of_mut;
use uefi::proto::console::text::Key;
use uefi::proto::unsafe_protocol;
use crate::simple_text_input_ex::{KeyState, RawKeyData, SimpleTextInputExProtocol};

#[derive(Debug)]
#[repr(transparent)]
#[unsafe_protocol(SimpleTextInputExProtocol::GUID)]
pub struct Input(SimpleTextInputExProtocol);

#[derive(Debug, Copy, Clone)]
pub struct KeyData {
    pub key: Key,
    pub key_state: KeyState,
}

impl Input {
    pub fn read_key_stroke_ex(&mut self) -> Option<KeyData> {
        let mut raw = RawKeyData::default();
        let this = addr_of_mut!(self.0);

        let status = unsafe {
            // FFI binding
            (self.0.read_key_stroke_ex)(this, &mut raw)
        };

        // Convert to Rust high-level type
        status.is_success().then_some(KeyData {
            key: Key::from(raw.key),
            // TODO: add new type enum for key state
            key_state: raw.key_state,
        })
    }
}