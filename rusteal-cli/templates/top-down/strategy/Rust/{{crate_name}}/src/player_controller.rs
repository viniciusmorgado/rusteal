// TopDownPlayerController: the Top Down template's
// `ATP_TopDownPlayerController` in Rust, point and click (or touch) controls:
// holding the input moves the character towards the cursor, a short press
// sends it there along the navigation mesh, with a cursor effect.
//
// The C++ class adds its mapping context and binds its input actions in
// `SetupInputComponent`, a C++ virtual; here that happens in
// `ReceiveBeginPlay`, by which time the local player and the controller's
// input component exist. The Blueprint child `BP_TopDownController` gives it
// the mapping context, the input actions and the cursor effect.

use bindings::ai::{AIBlueprintHelperLibrary, PathFollowingComponent};
use bindings::core_ue::EMouseCursor;
use bindings::engine::{
    ActorExt, ControllerExt, ETraceTypeQuery, FHitResultExt,
    GameplayStatics, PawnExt, PlayerController, PlayerControllerExt,
};
use bindings::enhanced_input::{
    ETriggerEvent, EnhancedInputLocalPlayerSubsystemExt, InputAction, InputMappingContext,
};
use bindings::input_core::ETouchIndex;
use bindings::niagara::{ENCPoolMethod, NiagaraFunctionLibrary, NiagaraSystem};
use bindings::prelude::*;
use glam::DVec3;
use rusteal_runtime::runtime::{
    LOG_DISPLAY, LOG_WARNING, OwnedStruct, Rotator, RustealResult, UObjectRef, ulog,
};
use rusteal_runtime::{uclass, uclass_impl};

#[uclass(parent = PlayerController)]
pub struct TopDownPlayerController {
    /// Component used for moving along a NavMesh path.
    #[component(name = "Path Following Component")]
    path_following_component: PathFollowingComponent,

    /// Time Threshold to know if it was a short press
    #[uproperty(EditAnywhere, category = "Input", default = 0.0)]
    short_press_threshold: f32,

    /// FX Class that we will spawn when clicking
    #[uproperty(EditAnywhere, name = "FXCursor", category = "Input")]
    fx_cursor: UObjectRef<NiagaraSystem>,

    /// MappingContext
    #[uproperty(EditAnywhere, category = "Input")]
    default_mapping_context: UObjectRef<InputMappingContext>,

    /// Jump Input Action
    #[uproperty(EditAnywhere, category = "Input")]
    set_destination_click_action: UObjectRef<InputAction>,

    /// Jump Input Action
    #[uproperty(EditAnywhere, category = "Input")]
    set_destination_touch_action: UObjectRef<InputAction>,

    /// Set to true if we're using touch input
    is_touch: bool,

    /// Saved location of the character movement destination
    cached_destination: DVec3,

    /// Time that the click input has been pressed
    follow_time: f32,
}

#[uclass_impl]
impl TopDownPlayerController {
    /// Everything `ATP_TopDownPlayerController::ATP_TopDownPlayerController()` sets.
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        // configure the controller
        let me = self.as_ref().checked()?;
        me.set_show_mouse_cursor(true);
        me.set_default_mouse_cursor(EMouseCursor::Default);
        Ok(())
    }

    /// Initialize input bindings
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        match self.setup_input() {
            Ok(true) => ulog!(LOG_DISPLAY, "[TopDown] input bound"),
            Ok(false) => {}
            Err(e) => ulog!(LOG_WARNING, "[TopDown] input setup failed: {e}"),
        }
    }

    /// Input handlers
    #[ufunction]
    fn on_input_started(&mut self) {
        if let Ok(me) = self.as_ref().checked() {
            me.stop_movement();
        }

        // Update the move destination to wherever the cursor is pointing at
        self.update_cached_destination();
    }

    #[ufunction]
    fn on_set_destination_triggered(&mut self) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        // We flag that the input is being pressed
        self.set_follow_time(self.follow_time() + GameplayStatics::get_world_delta_seconds(me.as_ref().upcast_to()) as f32);

        // Update the move destination to wherever the cursor is pointing at
        self.update_cached_destination();

        // Move towards mouse pointer or touch
        if let Ok(controlled_pawn) = me.k2_get_pawn().checked() {
            let world_direction =
                (self.cached_destination() - controlled_pawn.k2_get_actor_location().to_dvec3()).normalize_or_zero();
            controlled_pawn.add_movement_input(&FVector::from_dvec3(world_direction), Some(1.0), Some(false));
        }
    }

    #[ufunction]
    fn on_set_destination_released(&mut self) {
        // If it was a short press
        if self.follow_time() <= self.short_press_threshold() {
            // We move there and spawn some particles
            let me = self.as_ref();
            let destination = FVector::from_dvec3(self.cached_destination());
            AIBlueprintHelperLibrary::simple_move_to_location(me.upcast_to(), &destination);
            NiagaraFunctionLibrary::spawn_system_at_location(
                me.upcast_to(),
                self.fx_cursor(),
                &destination,
                &FRotator::from_rotator(Rotator::ZERO),
                &FVector::from_dvec3(DVec3::ONE),
                Some(true),
                Some(true),
                Some(ENCPoolMethod::None),
                Some(true),
            );
        }
        self.set_follow_time(0.0);
    }

    /// Triggered every frame when the input is held down
    #[ufunction]
    fn on_touch_triggered(&mut self) {
        self.set_is_touch(true);
        self.on_set_destination_triggered();
    }

    #[ufunction]
    fn on_touch_released(&mut self) {
        self.set_is_touch(false);
        self.on_set_destination_released();
    }
}

impl TopDownPlayerController {
    /// Everything `ATP_TopDownPlayerController::SetupInputComponent` does on a
    /// local player controller; `false` for any other.
    fn setup_input(&self) -> RustealResult<bool> {
        let me: UObjectRef<PlayerController> = self.as_ref();
        // Only set up input on local player controllers
        if !me.checked()?.is_local_player_controller() {
            return Ok(false);
        }

        // Add Input Mapping Context
        enhanced_input_subsystem(me)?
            .checked()?
            .add_mapping_context(self.default_mapping_context(), 0, &OwnedStruct::new());

        // Setup mouse input events
        let click = self.set_destination_click_action();
        bind_action(&me, click, ETriggerEvent::Started, "OnInputStarted")?;
        bind_action(&me, click, ETriggerEvent::Triggered, "OnSetDestinationTriggered")?;
        bind_action(&me, click, ETriggerEvent::Completed, "OnSetDestinationReleased")?;
        bind_action(&me, click, ETriggerEvent::Canceled, "OnSetDestinationReleased")?;

        // Setup touch input events
        let touch = self.set_destination_touch_action();
        bind_action(&me, touch, ETriggerEvent::Started, "OnInputStarted")?;
        bind_action(&me, touch, ETriggerEvent::Triggered, "OnTouchTriggered")?;
        bind_action(&me, touch, ETriggerEvent::Completed, "OnTouchReleased")?;
        bind_action(&me, touch, ETriggerEvent::Canceled, "OnTouchReleased")?;
        Ok(true)
    }

    /// Helper function to get the move destination
    fn update_cached_destination(&mut self) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        // We look for the location in the world where the player has pressed
        // the input, on the Visibility channel (TraceTypeQuery1)
        let (hit_successful, hit) = if self.is_touch() {
            me.get_hit_result_under_finger_by_channel(ETouchIndex::Touch1, ETraceTypeQuery::TraceTypeQuery1, true)
        } else {
            me.get_hit_result_under_cursor_by_channel(ETraceTypeQuery::TraceTypeQuery1, true)
        };

        // If we hit a surface, cache the location
        if hit_successful {
            self.set_cached_destination(hit.as_ref().get_location().to_dvec3());
        }
    }
}
