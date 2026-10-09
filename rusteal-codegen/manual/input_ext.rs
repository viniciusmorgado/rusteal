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
