// Copyright (c) Bemly, January 2026
// You may copy and distribute this file freely.  Any queries and
// complaints should be forwarded to bemly_@petalmail.com.
// If you make any changes to this file, please do not distribute
// the results under the name `bemly'.

use uefi::{Char16, Status};
use uefi::proto::console::text::Key;
use uefi_raw::protocol::console::InputKey;
use crate::simple_text_input_ex::*;

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
            key_state: raw.key_state,
        }
    }
}

impl KeyData {
    /// Create key data from char
    pub fn new(c: char) -> uefi::Result<Self> {
        let c = Char16::try_from(c).map_err(|_| Status::INVALID_PARAMETER)?;

        Ok(Self {
            key: Key::Printable(c),
            key_state: KeyState::default(),
        })
    }

    #[inline(always)]
    pub fn function_enable(&self) -> bool {
        (self.key_state.key_shift_state & SHIFT_STATE_VALID) != 0
    }

    #[inline(always)]
    pub fn r_shift(&self) -> bool {
        (self.key_state.key_shift_state & RIGHT_SHIFT_PRESSED) != 0
    }

    #[inline(always)]
    pub fn l_shift(&self) -> bool {
        (self.key_state.key_shift_state & LEFT_SHIFT_PRESSED) != 0
    }

    #[inline(always)]
     pub fn shift(&self) -> bool {
        const SHIFT_MASK: u32 = LEFT_SHIFT_PRESSED | RIGHT_SHIFT_PRESSED;
        (self.key_state.key_shift_state & SHIFT_MASK) != 0
    }

    #[inline(always)]
    pub fn r_ctrl(&self) -> bool {
        (self.key_state.key_shift_state & RIGHT_CONTROL_PRESSED) != 0
    }

    #[inline(always)]
    pub fn l_ctrl(&self) -> bool {
        (self.key_state.key_shift_state & LEFT_CONTROL_PRESSED) != 0
    }

    #[inline(always)]
    pub fn ctrl(&self) -> bool {
        const CTRL_MASK: u32 = LEFT_CONTROL_PRESSED | RIGHT_CONTROL_PRESSED;
        (self.key_state.key_shift_state & CTRL_MASK) != 0
    }

    #[inline(always)]
    pub fn r_alt(&self) -> bool {
        (self.key_state.key_shift_state & RIGHT_ALT_PRESSED) != 0
    }

    #[inline(always)]
    pub fn l_alt(&self) -> bool {
        (self.key_state.key_shift_state & LEFT_ALT_PRESSED) != 0
    }

    #[inline(always)]
    pub fn alt(&self) -> bool {
        const ALT_MASK: u32 = LEFT_ALT_PRESSED | RIGHT_ALT_PRESSED;
        (self.key_state.key_shift_state & ALT_MASK) != 0
    }

    #[inline(always)]
    pub fn r_logo(&self) -> bool {
        (self.key_state.key_shift_state & RIGHT_LOGO_PRESSED) != 0
    }

    #[inline(always)]
    pub fn l_logo(&self) -> bool {
        (self.key_state.key_shift_state & LEFT_LOGO_PRESSED) != 0
    }

    #[inline(always)]
    pub fn logo(&self) -> bool {
        const LOGO_MASK: u32 = LEFT_LOGO_PRESSED | RIGHT_LOGO_PRESSED;
        (self.key_state.key_shift_state & LOGO_MASK) != 0
    }

    #[inline(always)]
    pub fn menu(&self) -> bool {
        (self.key_state.key_shift_state & MENU_KEY_PRESSED) != 0
    }

    #[inline(always)]
    pub fn sys_req(&self) -> bool {
        (self.key_state.key_shift_state & SYS_REQ_PRESSED) != 0
    }

    #[inline(always)]
    pub fn toggle_enable(&self) -> bool {
        (self.key_state.key_toggle_state & TOGGLE_STATE_VALID) != 0
    }

    #[inline(always)]
    pub fn is_realtime(&self) -> bool {
        (self.key_state.key_toggle_state & KEY_STATE_EXPOSED) != 0
    }

    #[inline(always)]
    pub fn scroll_lock(&self) -> bool {
        (self.key_state.key_toggle_state & SCROLL_LOCK_ACTIVE) != 0
    }

    #[inline(always)]
    pub fn num_lock(&self) -> bool {
        (self.key_state.key_toggle_state & NUM_LOCK_ACTIVE) != 0
    }

    #[inline(always)]
    pub fn caps_lock(&self) -> bool {
        (self.key_state.key_toggle_state & CAPS_LOCK_ACTIVE) != 0
    }
}