use bindings::engine::{Actor, ControllerExt, Pawn, PlayerController};
use bindings::enhanced_input::{EnhancedInputLocalPlayerSubsystemExt, InputMappingContext};
use bindings::prelude::*;
use bindings::umg::{UserWidget, UserWidgetExt};
use rusteal_runtime::runtime::input::should_display_touch_interface;
use rusteal_runtime::runtime::{
    LOG_ERROR, LOG_WARNING, OwnedStruct, RustealResult, SubclassOf, UObjectRef, UeArray, ulog,
};
use rusteal_runtime::{uclass, uclass_impl};

use super::character::CombatCharacter;

#[uclass(parent = PlayerController)]
pub struct CombatPlayerController {
    #[uproperty(EditAnywhere)]
    default_mapping_contexts: UeArray<UObjectRef<InputMappingContext>>,

    #[uproperty(EditAnywhere)]
    mobile_excluded_mapping_contexts: UeArray<UObjectRef<InputMappingContext>>,

    #[uproperty(EditAnywhere)]
    mobile_controls_widget_class: SubclassOf<UserWidget>,

    #[uproperty]
    mobile_controls_widget: UObjectRef<UserWidget>,

    #[uproperty(EditAnywhere, default = false)]
    b_force_touch_controls: bool,

    #[uproperty(EditAnywhere)]
    character_class: SubclassOf<CombatCharacter>,

    #[uproperty]
    respawn_transform: OwnedStruct<FTransform>,
}

#[uclass_impl]
impl CombatPlayerController {
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let Err(e) = self.setup_input() {
            ulog!(LOG_WARNING, "[Combat] input mapping contexts failed: {e}");
        }
    }

    #[ufunction(Override)]
    fn receive_possess(&mut self, possessed_pawn: UObjectRef<Pawn>) {
        let controller: UObjectRef<PlayerController> = self.as_ref();

        let bound = possessed_pawn.checked().and_then(|pawn| {
            pawn.on_destroyed().add(move |destroyed_actor| {
                if let Ok(mut me) = CombatPlayerController::from_obj(controller) {
                    me.on_pawn_destroyed(destroyed_actor);
                }
            })
        });

        match bound {
            Ok(binding) => binding.detach(),
            Err(e) => ulog!(LOG_WARNING, "[Combat] cannot watch the pawn: {e}"),
        }
    }
}

impl CombatPlayerController {
    fn on_pawn_destroyed(&mut self, _destroyed_actor: UObjectRef<Actor>) {
        if let Err(e) = self.respawn() {
            ulog!(LOG_WARNING, "[Combat] respawn failed: {e}");
        }
    }

    fn respawn(&mut self) -> RustealResult<()> {
        let me = self.as_ref();

        let world = me.get_world()?;

        let respawned_character =
            world.spawn_actor_of_class(self.character_class(), &self.respawn_transform())?;

        me.checked()?.possess(respawned_character.upcast_to());

        Ok(())
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
