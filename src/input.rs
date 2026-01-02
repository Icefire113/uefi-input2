use core::ffi::c_void;
use core::ptr::addr_of_mut;
use uefi::{Char16, Event, Result, StatusExt};
use uefi::proto::console::text::{Key};
use uefi::proto::unsafe_protocol;
use uefi_raw::Status;
use crate::simple_text_input_ex::{KeyNotifyFunction, KeyState, KeyToggleState,
                                  RawKeyData, SimpleTextInputExProtocol, Boolean, InputKey};

/// height-level key data wrapper
#[derive(Debug, Copy, Clone)]
pub struct KeyData {
    pub key: Key,
    pub key_state: KeyState,
}

/// reverse conversion to C struct
impl From<KeyData> for RawKeyData {
    fn from(value: KeyData) -> Self {
        let input_key = match value.key {
            Key::Printable(c) => InputKey {
                scan_code: 0,
                unicode_char: u16::from(c),
            },
            Key::Special(code) => InputKey {
                scan_code: code.0,
                unicode_char: 0,
            },
        };

        Self {
            key: input_key,
            key_state: value.key_state,
        }
    }
}

/// forward conversion to Rust struct
impl From<RawKeyData> for KeyData {
    fn from(raw: RawKeyData) -> Self {
        Self {
            key: Key::from(raw.key),
            // TODO: add new type enum for key state
            key_state: raw.key_state,
        }
    }
}

impl KeyData {
    /// Create key data from char
    pub fn new(c: char) -> Result<Self> {
        let c = Char16::try_from(c).map_err(|_| Status::INVALID_PARAMETER)?;
        
        Ok(Self {
            key: Key::Printable(c),
            key_state: KeyState::default(),
        })
    }
}

/// safety wrapper for SimpleTextInputExProtocol
#[derive(Debug)]
#[repr(transparent)]
#[unsafe_protocol(SimpleTextInputExProtocol::GUID)]
pub struct Input(SimpleTextInputExProtocol);

impl Input {
    /// clear keyboard cache
    pub fn reset(&mut self, extended_verification: bool) -> Result {
        let this = addr_of_mut!(self.0);
        unsafe {
            (self.0.reset)(this, Boolean::from(extended_verification)).to_result()
        }
    }

    /// non-blocking keyboard read
    pub fn read_key_stroke_ex(&mut self) -> Option<KeyData> {
        let mut raw = RawKeyData::default();
        let this = addr_of_mut!(self.0);

        let status = unsafe {
            // FFI binding
            (self.0.read_key_stroke_ex)(this, &mut raw)
        };

        // Convert to Rust high-level type
        status.is_success().then_some(KeyData::from(raw))
    }

    /// Returns an event that is signaled when a key is pressed.
    ///
    /// This allows the caller to wait for input without polling in a busy loop,
    /// or to use it in `wait_for_event` along with other events (like timers).
    pub fn wait_for_key_event(&self) -> Option<Event> {
        unsafe { Event::from_ptr(self.0.wait_for_key_ex) }
    }

    pub fn set_state(&mut self, mut state: KeyToggleState) -> Result {
        let this = addr_of_mut!(self.0);
        unsafe {
            (self.0.set_state)(this, &mut state).to_result()
        }
    }

    /// Register a callback function to be invoked when a key is pressed.
    /// #### Usage
    /// 1. Prepare callback function
    /// ```rust,no_run
    /// extern "efiapi" fn listener(_key_data: *mut uefi_input2::simple_text_input_ex::RawKeyData) -> Status {
    ///     if _key_data.is_null() { return Status::INVALID_PARAMETER; }
    ///     let data = unsafe { *_key_data };
    ///     let data = KeyData::from(data);
    ///     uefi::println!("{:?}{:08X}", data.key, data.key_state.key_shift_state);
    ///     Status::SUCCESS
    /// }
    /// ```
    /// 2. Register callback function
    /// ```rust,no_run
    /// uefi_input2::with_stdin(|input| {
    ///     let trigger_key = KeyData::new('b')?;
    ///     let _listener = input.on_key_callback(&trigger_key, listener)?;
    ///     loop { core::hint::spin_loop() }
    ///     Ok(())
    ///  }).unwrap();
    /// ```
    pub fn on_key_callback(
        &mut self,
        key_data: &KeyData,
        notification_function: KeyNotifyFunction,
    ) -> Result<KeyNotifyHandle<'_>> {
        let this = addr_of_mut!(self.0);

        let mut handle = core::ptr::null_mut();
        let mut raw_data = RawKeyData::from(*key_data);

        unsafe {
            ((*this).register_key_notify)(
                this,
                &mut raw_data,
                notification_function,
                &mut handle,
            )
                .to_result_with_val(|| KeyNotifyHandle {
                    // the pointer needs to be cast back to a reference and stored in the Handle.
                    // ensure that the Handle can safety borrow the origin protocol.
                    proto: &mut *this,
                    handle,
                })
        }
    }
}

/// Keyboard notification handle
pub struct KeyNotifyHandle<'a> {
    /// Referencing the original agreement ensures that the agreement remains valid upon cancellation.
    proto: &'a mut SimpleTextInputExProtocol,
    /// The unique handle generated by UEFI
    handle: *mut c_void,
}

impl Drop for KeyNotifyHandle<'_> {
    /// Automatically unregister the handle when the user no longer needs it 
    /// (e.g. when the variable goes out of scope).
    fn drop(&mut self) {
        let this = addr_of_mut!(*self.proto);
        unsafe {
            let _ = ((*this).unregister_key_notify)(this, self.handle);
        }
    }
}
