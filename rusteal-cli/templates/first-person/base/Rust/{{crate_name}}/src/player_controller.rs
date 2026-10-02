// FirstPersonPlayerController: the First Person template's
// `ATP_FirstPersonPlayerController` in Rust.
//
// It registers the input mapping contexts its Blueprint child
// `BP_FirstPersonPlayerController` lists. The C++ class does that in
// `SetupInputComponent`, a C++ virtual; here it happens in `ReceiveBeginPlay`,
// by which time the local player and its Enhanced Input subsystem exist.
// On touch platforms, or with Force Touch Controls, it also spawns the mobile
// controls widget, as the C++ `BeginPlay` does. Its camera manager is
// `FirstPersonCameraManager`, which limits the look pitch.

use bindings::engine::{ControllerExt, PlayerController, PlayerControllerExt};
use bindings::enhanced_input::{EnhancedInputLocalPlayerSubsystemExt, InputMappingContext};
use bindings::prelude::*;
use bindings::umg::UserWidget;
use bindings::umg::UserWidgetExt;
use rusteal_runtime::runtime::input::should_display_touch_interface;
use rusteal_runtime::runtime::{
    LOG_DISPLAY, LOG_ERROR, LOG_WARNING, OwnedStruct, RustealResult, SubclassOf, UObjectRef,
    UeArray, ulog,
};
use rusteal_runtime::{uclass, uclass_impl};

use crate::camera_manager::FirstPersonCameraManager;

#[uclass(parent = PlayerController)]
pub struct FirstPersonPlayerController {
    /// Input Mapping Contexts
    #[uproperty(EditAnywhere)]
    default_mapping_contexts: UeArray<UObjectRef<InputMappingContext>>,

    /// Input Mapping Contexts
    #[uproperty(EditAnywhere)]
    mobile_excluded_mapping_contexts: UeArray<UObjectRef<InputMappingContext>>,

    /// Mobile controls widget to spawn
    #[uproperty(EditAnywhere)]
    mobile_controls_widget_class: SubclassOf<UserWidget>,

    /// Pointer to the mobile controls widget
    #[uproperty]
    mobile_controls_widget: UObjectRef<UserWidget>,

    /// If true, the player will use UMG touch controls even if not playing on mobile platforms
    #[uproperty(EditAnywhere, default = false)]
    b_force_touch_controls: bool,
}

#[uclass_impl]
impl FirstPersonPlayerController {
    /// Everything `ATP_FirstPersonPlayerController::ATP_FirstPersonPlayerController()` sets.
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        // set the player camera manager class
        self.as_ref()
            .checked()?
            .set_player_camera_manager_class(SubclassOf::<FirstPersonCameraManager>::base().upcast_to());
        Ok(())
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        match self.add_mapping_contexts() {
            Ok(None) => {}
            Ok(Some(0)) => ulog!(
                LOG_WARNING,
                "[FirstPerson] no input mapping contexts to add: list them in a Blueprint child \
                 (BP_FirstPersonPlayerController) and select it as the game mode's Player Controller Class"
            ),
            Ok(Some(count)) => {
                ulog!(LOG_DISPLAY, "[FirstPerson] {count} input mapping contexts added")
            }
            Err(e) => ulog!(LOG_WARNING, "[FirstPerson] input mapping contexts failed: {e}"),
        }

        // only spawn touch controls on local player controllers
        if let Err(e) = self.spawn_mobile_controls() {
            ulog!(LOG_ERROR, "[FirstPerson] Could not spawn mobile controls widget: {e}");
        }
    }
}

impl FirstPersonPlayerController {
    /// Everything `ATP_FirstPersonPlayerController::BeginPlay` does: spawn the
    /// mobile controls widget on a local controller that uses touch controls.
    fn spawn_mobile_controls(&mut self) -> RustealResult<()> {
        let me: UObjectRef<PlayerController> = self.as_ref();
        if !(me.checked()?.is_local_player_controller() && self.should_use_touch_controls()) {
            return Ok(());
        }

        // spawn the mobile controls widget
        let widget = create_widget_of_class(&me, self.mobile_controls_widget_class())?;

        // add the controls to the player screen
        widget.checked()?.add_to_player_screen(Some(0));
        self.set_mobile_controls_widget(widget);
        Ok(())
    }

    /// Everything `ATP_FirstPersonPlayerController::SetupInputComponent` does; the
    /// number of contexts added, `None` for a controller that is not local.
    fn add_mapping_contexts(&self) -> RustealResult<Option<usize>> {
        let me: UObjectRef<PlayerController> = self.as_ref();
        // only add IMCs for local player controllers
        if !me.checked()?.is_local_controller() {
            return Ok(None);
        }

        // Add Input Mapping Contexts
        let subsystem = enhanced_input_subsystem(me)?.checked()?;
        let mut contexts = self.default_mapping_contexts().to_vec()?;

        // only add these IMCs if we're not using mobile touch input
        if !self.should_use_touch_controls() {
            contexts.extend(self.mobile_excluded_mapping_contexts().to_vec()?);
        }

        let count = contexts.len();
        for context in contexts {
            subsystem.add_mapping_context(context, 0, &OwnedStruct::new());
        }
        Ok(Some(count))
    }

    /// Returns true if the player should use UMG touch controls
    fn should_use_touch_controls(&self) -> bool {
        // are we on a mobile platform? Should we force touch?
        should_display_touch_interface() || self.b_force_touch_controls()
    }
}
