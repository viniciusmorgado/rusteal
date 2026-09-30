// Enhanced Input bindings (raw handle versions).
// The typed wrapper lives in the generated bindings' manual/input_ext.rs.

use rusteal_ffi::UObjectHandle;

use crate::error::{check_ffi_ctx, RustealResult};
use crate::ffi_dispatch;

/// Bind `trigger_event` (an `ETriggerEvent` value) of `action` on `actor`'s
/// Enhanced Input component to `actor`'s UFUNCTION `function` (its UE name),
/// which takes no parameters or one `FInputActionValue`: the C++
/// `EnhancedInputComponent->BindAction(Action, TriggerEvent, this, &AFoo::Function)`.
///
/// The actor must have its input component, which a pawn gets when a local
/// player possesses it: bind from `ReceiveRestarted`, as C++ binds from
/// `SetupPlayerInputComponent`.
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
