// Type-safe Enhanced Input binding on top of rusteal_core::input.

use rusteal_core::{RustealResult, SubclassOf, UObjectRef, UStructRef, UeClass};

use crate::engine::{LocalPlayerSubsystem, PlayerController, SubsystemBlueprintLibrary};
use crate::enhanced_input::{
    ETriggerEvent, EnhancedInputLibrary, EnhancedInputLocalPlayerSubsystem, FInputActionValue,
    InputAction,
};

/// The value an input action handler receives, read as the C++ template
/// `FInputActionValue::Get<T>()` reads it (not reflected, hence by hand).
pub trait InputActionValueExt {
    /// `Value.Get<bool>()`: whether the value is non-zero.
    fn get_bool(&self) -> bool;
    /// `Value.Get<float>()`.
    fn axis1d(&self) -> f64;
    /// `Value.Get<FVector2D>()`.
    fn axis2d(&self) -> glam::DVec2;
    /// `Value.Get<FVector>()`.
    fn axis3d(&self) -> glam::DVec3;
}

impl InputActionValueExt for UStructRef<FInputActionValue> {
    fn get_bool(&self) -> bool {
        self.axis3d() != glam::DVec3::ZERO
    }

    fn axis1d(&self) -> f64 {
        self.axis3d().x
    }

    fn axis2d(&self) -> glam::DVec2 {
        self.axis3d().truncate()
    }

    fn axis3d(&self) -> glam::DVec3 {
        let (x, y, z, _) =
            EnhancedInputLibrary::break_input_action_value(
                &self.to_owned(),
            );
        glam::DVec3::new(x, y, z)
    }
}

/// Bind `trigger_event` of `action` on `actor`'s Enhanced Input component to
/// the actor's UFUNCTION named `function`, which takes no parameters or one
/// `UStructRef<FInputActionValue>`: the C++
/// `EnhancedInputComponent->BindAction(Action, TriggerEvent, this, &AFoo::Function)`.
///
/// `function` is the UE name: `"DoMove"` for `#[ufunction] fn do_move`, or an
/// engine function such as `"Jump"`. The actor must have its input component,
/// which a pawn gets when a local player possesses it: bind from
/// `receive_restarted`, as C++ binds from `SetupPlayerInputComponent`.
/// Binding the same function to the same action and event on the same
/// component again does nothing, so `receive_restarted` may run more than once.
pub fn bind_action(
    actor: &UObjectRef<impl UeClass>,
    action: UObjectRef<InputAction>,
    trigger_event: ETriggerEvent,
    function: &str,
) -> RustealResult<()> {
    let actor = actor.checked()?.raw();
    let action = action.checked()?.raw();
    rusteal_core::input::bind_action_raw(actor, action, trigger_event as u8, function)
}

/// The Enhanced Input subsystem of `controller`'s local player, where mapping
/// contexts are added: the C++
/// `ULocalPlayer::GetSubsystem<UEnhancedInputLocalPlayerSubsystem>(GetLocalPlayer())`.
/// Fails for a controller without a local player (a remote or AI one).
pub fn enhanced_input_subsystem(
    controller: UObjectRef<PlayerController>,
) -> RustealResult<UObjectRef<EnhancedInputLocalPlayerSubsystem>> {
    controller.checked()?;
    let class: SubclassOf<LocalPlayerSubsystem> =
        SubclassOf::<EnhancedInputLocalPlayerSubsystem>::base().upcast();
    SubsystemBlueprintLibrary::get_local_player_sub_system_from_player_controller(
        controller, class,
    )
    .cast::<EnhancedInputLocalPlayerSubsystem>()
}
