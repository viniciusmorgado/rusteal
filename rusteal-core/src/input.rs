use rusteal_ffi::UObjectHandle;

use crate::error::{RustealResult, check_ffi_ctx};
use crate::ffi_dispatch;

pub fn bind_action_raw(
    actor: UObjectHandle,
    action: UObjectHandle,
    trigger_event: u8,
    function: &str,
) -> RustealResult<()> {
    check_ffi_ctx(
        unsafe {
            ffi_dispatch::input_bind_action(
                actor,
                action,
                trigger_event,
                function.as_ptr(),
                function.len() as u32,
            )
        },
        function,
    )
}

pub fn should_display_touch_interface() -> bool {
    unsafe { ffi_dispatch::input_should_display_touch_interface() }
}
