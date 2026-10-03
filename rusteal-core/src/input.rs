// Enhanced Input bindings (raw handle versions) and the touch interface check.
// The typed wrapper lives in the generated bindings' manual/input_ext.rs.

use rusteal_ffi::UObjectHandle;

use crate::error::{check_ffi_ctx, RustealResult};
use crate::ffi_dispatch;

/// Bind `trigger_event` (an `ETriggerEvent` value) of `action` on `actor`'s
/// Enhanced Input component to `actor`'s UFUNCTION `function` (its UE name),
/// which takes no parameters, or `(FInputActionValue, f32 elapsed seconds,
/// f32 triggered seconds, InputAction)` or a leading part of it: the C++
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

/// Whether the platform shows touch controls: the C++
/// `SVirtualJoystick::ShouldDisplayTouchInterface()`, true on mobile, with
/// `bAlwaysShowTouchInterface` in the input settings, or when faked touch
/// events (`Use Mouse for Touch`) are set to display them.
pub fn should_display_touch_interface() -> bool {
    unsafe { ffi_dispatch::input_should_display_touch_interface() }
}
