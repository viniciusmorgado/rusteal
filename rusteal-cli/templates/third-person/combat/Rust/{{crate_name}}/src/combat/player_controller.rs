// CombatPlayerController: the Combat variant's `ACombatPlayerController` in
// Rust. Manages the input mappings and the touch controls like the Third
// Person controller, and respawns the player character at the last
// checkpoint (its respawn transform) when it is destroyed.
//
// `SetupInputComponent` and `OnPossess` are C++ virtuals; here their work
// happens in `ReceiveBeginPlay` and `ReceivePossess`. The Blueprint child
// `BP_CombatPlayerController` lists the mapping contexts, the mobile controls
// widget and the character class to respawn.

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
    /// Input mapping context for this player
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

    /// Character class to respawn when the possessed pawn is destroyed
    #[uproperty(EditAnywhere)]
    character_class: SubclassOf<CombatCharacter>,

    /// Transform to respawn the character at. Can be set to create
    /// checkpoints (`set_respawn_transform`)
    #[uproperty]
    respawn_transform: OwnedStruct<FTransform>,
}

#[uclass_impl]
impl CombatPlayerController {
    /// Initialize input bindings
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let Err(e) = self.setup_input() {
            ulog!(LOG_WARNING, "[Combat] input mapping contexts failed: {e}");
        }
    }

    /// Pawn initialization
    #[ufunction(Override)]
    fn receive_possess(&mut self, possessed_pawn: UObjectRef<Pawn>) {
        // subscribe to the pawn's OnDestroyed delegate
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
    /// Called if the possessed pawn is destroyed
    fn on_pawn_destroyed(&mut self, _destroyed_actor: UObjectRef<Actor>) {
        if let Err(e) = self.respawn() {
            ulog!(LOG_WARNING, "[Combat] respawn failed: {e}");
        }
    }

    fn respawn(&mut self) -> RustealResult<()> {
        let me = self.as_ref();
        // spawn a new character at the respawn transform
        let world = me.get_world()?;
        let respawned_character = world.spawn_actor_of_class(self.character_class(), &self.respawn_transform())?;
        // possess the character
        me.checked()?.possess(respawned_character.upcast_to());
        Ok(())
    }

    /// `SetupInputComponent` and `BeginPlay`: the mapping contexts, then the
    /// touch controls, on a local player controller.
    fn setup_input(&mut self) -> RustealResult<()> {
        let me: UObjectRef<PlayerController> = self.as_ref();
        // only add IMCs for local player controllers
        if !me.checked()?.is_local_player_controller() {
            return Ok(());
        }

        // add the input mapping context
        let subsystem = enhanced_input_subsystem(me)?.checked()?;
        let mut contexts = self.default_mapping_contexts().to_vec()?;
        // only add these IMCs if we're not using mobile touch input
        if !self.should_use_touch_controls() {
            contexts.extend(self.mobile_excluded_mapping_contexts().to_vec()?);
        }
        for context in contexts {
            subsystem.add_mapping_context(context, 0, &OwnedStruct::new());
        }

        // only spawn touch controls on local player controllers
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
