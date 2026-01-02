use core::ffi::c_void;
use uefi_raw::{guid, Boolean, Event, Guid, Status};
use uefi_raw::protocol::console::InputKey;

pub type KeyToggleState = u8;
/// Keyboard states:
/// including Shift/Alt/Ctrl states and Caps Lock flags.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct KeyState {
    /// The bitmask here is determined using the SHIFT_STATE constant below.
    pub key_shift_state: u32,
    /// TOGGLE_STATE
    pub key_toggle_state: KeyToggleState,
}

/// Complete key data structure
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct KeyData {
    pub key: InputKey,
    pub key_state: KeyState,
}

/// Protocol constant definition
/// Key combination status mask
pub const SHIFT_STATE_VALID: u32     = 0x8000_0000;
pub const RIGHT_SHIFT_PRESSED: u32   = 0x0000_0001;
pub const LEFT_SHIFT_PRESSED: u32    = 0x0000_0002;
pub const RIGHT_CONTROL_PRESSED: u32 = 0x0000_0004;
pub const LEFT_CONTROL_PRESSED: u32  = 0x0000_0008;
pub const RIGHT_ALT_PRESSED: u32     = 0x0000_0010;
pub const LEFT_ALT_PRESSED: u32      = 0x0000_0020;
pub const RIGHT_LOGO_PRESSED: u32    = 0x0000_0040;
pub const LEFT_LOGO_PRESSED: u32     = 0x0000_0080;
pub const MENU_KEY_PRESSED: u32      = 0x0000_0100;
pub const SYS_REQ_PRESSED: u32       = 0x0000_0200;

/// Toggle state mask
pub const TOGGLE_STATE_VALID: u8     = 0x80;
pub const KEY_STATE_EXPOSED: u8      = 0x40;
pub const SCROLL_LOCK_ACTIVE: u8     = 0x01;
pub const NUM_LOCK_ACTIVE: u8        = 0x02;
pub const CAPS_LOCK_ACTIVE: u8       = 0x04;

/// Protocol interface definition
/// Key notification callback function type
pub type KeyNotifyFunction = unsafe extern "efiapi" fn(key_data: *mut KeyData) -> Status;

/// EFI_SIMPLE_TEXT_INPUT_EX_PROTOCOL
/// Extended input protocol that allows obtaining modifier key (Shift/Alt/Ctrl) states.
#[derive(Debug)]
#[repr(C)]
pub struct SimpleTextInputExProtocol {
    /// Resets the input device hardware.
    pub reset: unsafe extern "efiapi" fn(this: *mut Self, extended_verification: Boolean) -> Status,

    /// Reads key data (including KeyState).
    pub read_key_stroke_ex: unsafe extern "efiapi" fn(this: *mut Self, key_data: *mut KeyData) -> Status,

    /// Event for waiting for a key press.
    pub wait_for_key_ex: Event,

    /// Sets the keyboard indicator light state (e.g., CapsLock).
    pub set_state: unsafe extern "efiapi" fn(this: *mut Self, key_toggle_state: *mut KeyToggleState) -> Status,

    /// Registers a key notification function, triggered when a specific key is pressed.
    pub register_key_notify: unsafe extern "efiapi" fn(
        this: *mut Self,
        key_data: *mut KeyData,
        key_notification_function: KeyNotifyFunction,
        notify_handle: *mut *mut c_void,
    ) -> Status,

    /// Unregisters a key notification.
    pub unregister_key_notify: unsafe extern "efiapi" fn(this: *mut Self, notification_handle: *mut c_void) -> Status,
}

impl SimpleTextInputExProtocol {
    pub const GUID: Guid = guid!("dd9e7534-7762-4698-8c14-f58517a625aa");
}