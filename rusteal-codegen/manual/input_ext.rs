use rusteal_core::{RustealResult, SubclassOf, UObjectRef, UStructRef, UeClass};

use crate::engine::{LocalPlayerSubsystem, PlayerController, SubsystemBlueprintLibrary};
use crate::enhanced_input::{
    ETriggerEvent, EnhancedInputLibrary, EnhancedInputLocalPlayerSubsystem, FInputActionValue,
    InputAction,
};

pub trait InputActionValueExt {
    fn get_bool(&self) -> bool;
    fn axis1d(&self) -> f64;
    fn axis2d(&self) -> glam::DVec2;
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

/// A binding made by [`bind_action`]: it lasts as long as the actor's input component keeps it
/// (a pawn's lasts until it is unpossessed); [`ActionBinding::unbind`] removes it sooner.
#[derive(Debug, Clone, Copy)]
pub struct ActionBinding {
    actor: rusteal_ffi::UObjectHandle,
    handle: u32,
}

impl ActionBinding {
    pub fn unbind(self) {
        rusteal_core::input::unbind_action_raw(self.actor, self.handle);
    }
}

/// Calls `callback` with the action's value each time `trigger_event` happens on `action`, for
/// `actor`'s Enhanced Input component; `method(self, Self::look)` makes the callback a method of
/// the class.
#[track_caller]
pub fn bind_action(
    actor: &UObjectRef<impl UeClass>,
    action: UObjectRef<InputAction>,
    trigger_event: ETriggerEvent,
    mut callback: impl FnMut(UStructRef<FInputActionValue>) + Send + 'static,
) -> RustealResult<ActionBinding> {
    let actor = actor.checked()?.raw();
    let action = action.checked()?.raw();

    let handle = rusteal_core::input::bind_action_closure_raw(
        actor,
        action,
        trigger_event as u8,
        move |args| {
            let value = unsafe { rusteal_core::struct_ref_from_param(args.value as *mut u8, 0) };
            callback(value);
        },
    )?;

    Ok(ActionBinding { actor, handle })
}

/// Binds the UFunction named `function` of `actor`, as Blueprint's input events do: for a
/// function a Blueprint defines. Rust code binds a closure or a method with [`bind_action`].
#[track_caller]
pub fn bind_action_by_name(
    actor: &UObjectRef<impl UeClass>,
    action: UObjectRef<InputAction>,
    trigger_event: ETriggerEvent,
    function: &str,
) -> RustealResult<()> {
    let actor = actor.checked()?.raw();
    let action = action.checked()?.raw();
    rusteal_core::input::bind_action_raw(actor, action, trigger_event as u8, function)
}

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
