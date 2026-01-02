use uefi::boot::{get_handle_for_protocol, open_protocol_exclusive};
use uefi::proto::unsafe_protocol;
use crate::simple_text_input_ex::{KeyData, SimpleTextInputExProtocol};
use uefi::{Result};

#[derive(Debug)]
#[repr(transparent)]
#[unsafe_protocol(SimpleTextInputExProtocol::GUID)]
pub struct Input(SimpleTextInputExProtocol);

impl Input {
    pub fn read_key_stroke_ex(&mut self) -> Option<KeyData> {
        let mut key_data = KeyData::default();
        let this = self as *mut Input as *mut SimpleTextInputExProtocol;

        let status = unsafe {
            (self.0.read_key_stroke_ex)(this, &mut key_data)
        };
        status.is_success().then_some(key_data)
    }
}


pub fn with_stdin<F, R>(mut f: F) -> Result<R>
where
    F: FnMut(&mut Input) -> Result<R>
{
    let input = get_handle_for_protocol::<Input>()?;
    let mut input = open_protocol_exclusive::<Input>(input)?;

    f(&mut *input)
}