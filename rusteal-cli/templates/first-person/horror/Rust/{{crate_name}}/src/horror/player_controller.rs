// HorrorPlayerController: the Horror variant's `AHorrorPlayerController` in
// Rust. Manages the input mappings and the touch controls like the First
// Person controller, and the UI that shows its character's sprint meter.
//
// `SetupInputComponent` and `OnPossess` are C++ virtuals; here their work
// happens in `ReceiveBeginPlay` and `ReceivePossess`. The Blueprint child
// `BP_HorrorPlayerController` lists the mapping contexts, the mobile controls
// widget and the UI widget class (`UI_Horror`).

use bindings::engine::{ControllerExt, Pawn, PlayerController, PlayerControllerExt};
use bindings::enhanced_input::{EnhancedInputLocalPlayerSubsystemExt, InputMappingContext};
use bindings::prelude::*;
use bindings::umg::{UserWidget, UserWidgetExt};
use rusteal_runtime::runtime::input::should_display_touch_interface;
use rusteal_runtime::runtime::{
    LOG_ERROR, LOG_WARNING, OwnedStruct, RustealResult, SubclassOf, UObjectRef, UeArray, ulog,
};
use rusteal_runtime::{uclass, uclass_impl};

use super::character::HorrorCharacter;
use super::ui::HorrorUI;
use crate::camera_manager::FirstPersonCameraManager;

#[uclass(parent = PlayerController)]
pub struct HorrorPlayerController {
    /// Type of UI widget to spawn
    #[uproperty(EditAnywhere, name = "HorrorUIClass", category = "Horror|UI")]
    horror_ui_class: SubclassOf<HorrorUI>,

    /// Pointer to the UI widget
    #[uproperty(name = "HorrorUI")]
    horror_ui: UObjectRef<HorrorUI>,

    /// Input Mapping Contexts
    #[uproperty(EditAnywhere, category = "Input|Input Mappings")]
    default_mapping_contexts: UeArray<UObjectRef<InputMappingContext>>,

    /// Input Mapping Contexts
    #[uproperty(EditAnywhere, category = "Input|Input Mappings")]
    mobile_excluded_mapping_contexts: UeArray<UObjectRef<InputMappingContext>>,

    /// Mobile controls widget to spawn
    #[uproperty(EditAnywhere, category = "Input|Touch Controls")]
    mobile_controls_widget_class: SubclassOf<UserWidget>,

    /// Pointer to the mobile controls widget
    #[uproperty]
    mobile_controls_widget: UObjectRef<UserWidget>,

    /// If true, the player will use UMG touch controls even if not playing on mobile platforms
    #[uproperty(EditAnywhere, category = "Input|Touch Controls", default = false)]
    b_force_touch_controls: bool,
}

#[uclass_impl]
impl HorrorPlayerController {
    /// Everything `AHorrorPlayerController::AHorrorPlayerController()` sets.
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        // set the player camera manager class
        self.as_ref()
            .checked()?
            .set_player_camera_manager_class(SubclassOf::<FirstPersonCameraManager>::base().upcast_to());
        Ok(())
    }

    /// Input mapping context setup
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let Err(e) = self.setup_input() {
            ulog!(LOG_WARNING, "[Horror] input mapping contexts failed: {e}");
        }
    }

    /// Possessed pawn initialization
    #[ufunction(Override)]
    fn receive_possess(&mut self, possessed_pawn: UObjectRef<Pawn>) {
        if let Err(e) = self.setup_ui(possessed_pawn) {
            ulog!(LOG_WARNING, "[Horror] the UI failed: {e}");
        }
    }
}

impl HorrorPlayerController {
    /// Everything `AHorrorPlayerController::OnPossess` does: show the UI and
    /// make it the character's sprint listener.
    fn setup_ui(&mut self, possessed_pawn: UObjectRef<Pawn>) -> RustealResult<()> {
        let me: UObjectRef<PlayerController> = self.as_ref();
        // only spawn UI on local player controllers
        if !me.checked()?.is_local_player_controller() {
            return Ok(());
        }

        // set up the UI for the character
        let Ok(horror_character) = HorrorCharacter::from_obj(possessed_pawn) else {
            return Ok(());
        };

        // create the UI
        if !self.horror_ui().is_valid() {
            let widget = create_widget_of_class(&me, self.horror_ui_class())?;
            widget.checked()?.add_to_player_screen(Some(0));
            self.set_horror_ui(widget);
        }
        HorrorUI::from_obj(self.horror_ui())?.setup_character(&horror_character)
    }

    /// `SetupInputComponent`: the mapping contexts, then the touch controls,
    /// on a local player controller.
    fn setup_input(&mut self) -> RustealResult<()> {
        let me: UObjectRef<PlayerController> = self.as_ref();
        // only add IMCs for local player controllers
        if !me.checked()?.is_local_player_controller() {
            return Ok(());
        }

        // Add Input Mapping Contexts
        let subsystem = enhanced_input_subsystem(me)?.checked()?;
        let mut contexts = self.default_mapping_contexts().to_vec()?;
        // only add these IMCs if we're not using mobile touch input
        if !self.should_use_touch_controls() {
            contexts.extend(self.mobile_excluded_mapping_contexts().to_vec()?);
        }
        for context in contexts {
            subsystem.add_mapping_context(context, 0, &OwnedStruct::new());
        }

        if self.should_use_touch_controls() {
            // spawn the mobile controls widget
            match create_widget_of_class(&me, self.mobile_controls_widget_class()) {
                Ok(widget) => {
                    // add the controls to the player screen
                    widget.checked()?.add_to_player_screen(Some(0));
                    self.set_mobile_controls_widget(widget);
                }
                Err(_) => ulog!(LOG_ERROR, "Could not spawn mobile controls widget."),
            }
        }
        Ok(())
    }

    /// Returns true if the player should use UMG touch controls
    fn should_use_touch_controls(&self) -> bool {
        // are we on a mobile platform? Should we force touch?
        should_display_touch_interface() || self.b_force_touch_controls()
    }
}
