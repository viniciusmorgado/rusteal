use bindings::engine::{
    Actor, ActorExt, ControllerExt, GameplayStatics, Pawn, PlayerController, PlayerStart,
};
use bindings::enhanced_input::{EnhancedInputLocalPlayerSubsystemExt, InputMappingContext};
use bindings::prelude::*;
use bindings::umg::{UserWidget, UserWidgetExt};
use rusteal_runtime::runtime::input::should_display_touch_interface;
use rusteal_runtime::runtime::{
    LOG_ERROR, LOG_WARNING, OwnedStruct, RustealResult, SubclassOf, UObjectRef, UeArray, ulog,
};
use rusteal_runtime::{uclass, uclass_impl};

use super::character::TwinStickCharacter;

#[uclass(parent = PlayerController)]
pub struct TwinStickPlayerController {
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

    #[uproperty(EditAnywhere, category = "Respawn")]
    character_class: SubclassOf<TwinStickCharacter>,
}

#[uclass_impl]
impl TwinStickPlayerController {
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let Err(e) = self.setup_input() {
            ulog!(
                LOG_WARNING,
                "[TwinStick] input mapping contexts failed: {e}"
            );
        }
    }

    #[ufunction(Override)]
    fn receive_possess(&mut self, possessed_pawn: UObjectRef<Pawn>) {
        let controller: UObjectRef<PlayerController> = self.as_ref();

        let bound = possessed_pawn.checked().and_then(|pawn| {
            pawn.on_destroyed().add(move |destroyed_actor| {
                if let Ok(mut me) = TwinStickPlayerController::from_obj(controller) {
                    me.on_pawn_destroyed(destroyed_actor);
                }
            })
        });

        match bound {
            Ok(binding) => binding.detach(),
            Err(e) => ulog!(LOG_WARNING, "[TwinStick] cannot watch the pawn: {e}"),
        }
    }
}

impl TwinStickPlayerController {
    fn on_pawn_destroyed(&mut self, _destroyed_actor: UObjectRef<Actor>) {
        if let Err(e) = self.respawn() {
            ulog!(LOG_WARNING, "[TwinStick] respawn failed: {e}");
        }
    }

    fn respawn(&mut self) -> RustealResult<()> {
        let me = self.as_ref();

        let player_starts = GameplayStatics::get_all_actors_of_class(
            me.upcast_to(),
            SubclassOf::<PlayerStart>::base().upcast_to(),
        );

        let Some(player_start) = player_starts.first() else {
            return Ok(());
        };

        let transform = player_start.checked()?.get_transform();

        let respawned_character = me
            .get_world()?
            .spawn_actor_of_class(self.character_class(), &transform)?;

        me.checked()?.possess(respawned_character.upcast_to());

        Ok(())
    }

    fn setup_input(&mut self) -> RustealResult<()> {
        let me: UObjectRef<PlayerController> = self.as_ref();

        if !me.checked()?.is_local_player_controller() {
            return Ok(());
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

        let subsystem = enhanced_input_subsystem(me)?.checked()?;
        let mut contexts = self.default_mapping_contexts().to_vec()?;

        if !self.should_use_touch_controls() {
            contexts.extend(self.mobile_excluded_mapping_contexts().to_vec()?);
        }

        for context in contexts {
            subsystem.add_mapping_context(context, 0, &OwnedStruct::new());
        }

        Ok(())
    }

    fn should_use_touch_controls(&self) -> bool {
        should_display_touch_interface() || self.b_force_touch_controls()
    }
}
