use rusteal_ffi::{RustealInputArgs, UObjectHandle};

use crate::delegate_registry::{register_callback, unregister_callback};
use crate::error::{RustealResult, check_ffi, check_ffi_ctx};
use crate::ffi_dispatch;

#[track_caller]
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

/// Binds `callback` to an input action of `actor`'s Enhanced Input component: called with the
/// action's state each time `trigger_event` happens. The closure is freed when the component
/// drops the binding (when the pawn is unpossessed, say) or [`unbind_action_raw`] removes it.
#[track_caller]
pub fn bind_action_closure_raw(
    actor: UObjectHandle,
    action: UObjectHandle,
    trigger_event: u8,
    mut callback: impl FnMut(&RustealInputArgs) + Send + 'static,
) -> RustealResult<u32> {
    let id = register_callback(move |params| {
        if !params.is_null() {
            callback(unsafe { &*(params as *const RustealInputArgs) });
        }
    });

    let mut handle = 0u32;

    let bound = check_ffi(unsafe {
        ffi_dispatch::input_bind_action_closure(actor, action, trigger_event, id, &mut handle)
    });

    if let Err(e) = bound {
        unregister_callback(id);
        return Err(e);
    }

    Ok(handle)
}

pub fn unbind_action_raw(actor: UObjectHandle, handle: u32) {
    if crate::api::is_api_initialized() {
        let _ = unsafe { ffi_dispatch::input_unbind_action(actor, handle) };
    }
}
