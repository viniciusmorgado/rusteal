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
    #[uproperty(EditAnywhere, name = "HorrorUIClass", category = "Horror|UI")]
    horror_ui_class: SubclassOf<HorrorUI>,

    #[uproperty(name = "HorrorUI")]
    horror_ui: UObjectRef<HorrorUI>,

    #[uproperty(EditAnywhere, category = "Input|Input Mappings")]
    default_mapping_contexts: UeArray<UObjectRef<InputMappingContext>>,

    #[uproperty(EditAnywhere, category = "Input|Input Mappings")]
    mobile_excluded_mapping_contexts: UeArray<UObjectRef<InputMappingContext>>,

    #[uproperty(EditAnywhere, category = "Input|Touch Controls")]
    mobile_controls_widget_class: SubclassOf<UserWidget>,

    #[uproperty]
    mobile_controls_widget: UObjectRef<UserWidget>,

    #[uproperty(EditAnywhere, category = "Input|Touch Controls", default = false)]
    b_force_touch_controls: bool,
}

#[uclass_impl]
impl HorrorPlayerController {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        self.as_ref().checked()?.set_player_camera_manager_class(
            SubclassOf::<FirstPersonCameraManager>::base().upcast_to(),
        );

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let Err(e) = self.setup_input() {
            ulog!(LOG_WARNING, "[Horror] input mapping contexts failed: {e}");
        }
    }

    #[ufunction(Override)]
    fn receive_possess(&mut self, possessed_pawn: UObjectRef<Pawn>) {
        if let Err(e) = self.setup_ui(possessed_pawn) {
            ulog!(LOG_WARNING, "[Horror] the UI failed: {e}");
        }
    }
}

impl HorrorPlayerController {
    fn setup_ui(&mut self, possessed_pawn: UObjectRef<Pawn>) -> RustealResult<()> {
        let me: UObjectRef<PlayerController> = self.as_ref();

        if !me.checked()?.is_local_player_controller() {
            return Ok(());
        }

        let Ok(horror_character) = HorrorCharacter::from_obj(possessed_pawn) else {
            return Ok(());
        };

        if !self.horror_ui().is_valid() {
            let widget = create_widget_of_class(&me, self.horror_ui_class())?;
            widget.checked()?.add_to_player_screen(Some(0));
            self.set_horror_ui(widget);
        }

        HorrorUI::from_obj(self.horror_ui())?.setup_character(&horror_character)
    }

    fn setup_input(&mut self) -> RustealResult<()> {
        let me: UObjectRef<PlayerController> = self.as_ref();

        if !me.checked()?.is_local_player_controller() {
            return Ok(());
        }

        let subsystem = enhanced_input_subsystem(me)?.checked()?;
        let mut contexts = self.default_mapping_contexts().to_vec()?;

        if !self.should_use_touch_controls() {
            contexts.extend(self.mobile_excluded_mapping_contexts().to_vec()?);
        }

        for context in contexts {
            subsystem.add_mapping_context(context, 0, &OwnedStruct::new());
        }

        if self.should_use_touch_controls() {
            match create_widget_of_class(&me, self.mobile_controls_widget_class()) {
                Ok(widget) => {
                    widget.checked()?.add_to_player_screen(Some(0));
                    self.set_mobile_controls_widget(widget);
                }
                Err(_) => ulog!(LOG_ERROR, "Could not spawn mobile controls widget."),
            }
        }

        Ok(())
    }

    fn should_use_touch_controls(&self) -> bool {
        should_display_touch_interface() || self.b_force_touch_controls()
    }
}
