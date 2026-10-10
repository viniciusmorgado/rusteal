use bindings::ai::{AIBlueprintHelperLibrary, PathFollowingComponent};
use bindings::core_ue::EMouseCursor;
use bindings::engine::{
    ActorExt, ControllerExt, ETraceTypeQuery, FHitResultExt, GameplayStatics, PawnExt,
    PlayerController, PlayerControllerExt,
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
    #[component(name = "Path Following Component")]
    path_following_component: PathFollowingComponent,

    #[uproperty(EditAnywhere, category = "Input", default = 0.0)]
    short_press_threshold: f32,

    #[uproperty(EditAnywhere, name = "FXCursor", category = "Input")]
    fx_cursor: UObjectRef<NiagaraSystem>,

    #[uproperty(EditAnywhere, category = "Input")]
    default_mapping_context: UObjectRef<InputMappingContext>,

    #[uproperty(EditAnywhere, category = "Input")]
    set_destination_click_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    set_destination_touch_action: UObjectRef<InputAction>,

    is_touch: bool,

    cached_destination: DVec3,

    follow_time: f32,
}

#[uclass_impl]
impl TopDownPlayerController {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;
        me.set_show_mouse_cursor(true);
        me.set_default_mouse_cursor(EMouseCursor::Default);

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        match self.setup_input() {
            Ok(true) => ulog!(LOG_DISPLAY, "[TopDown] input bound"),
            Ok(false) => {}
            Err(e) => ulog!(LOG_WARNING, "[TopDown] input setup failed: {e}"),
        }
    }

    #[ufunction]
    fn on_input_started(&mut self) {
        if let Ok(me) = self.as_ref().checked() {
            me.stop_movement();
        }

        self.update_cached_destination();
    }

    #[ufunction]
    fn on_set_destination_triggered(&mut self) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        self.set_follow_time(
            self.follow_time()
                + GameplayStatics::get_world_delta_seconds(me.as_ref().upcast_to()) as f32,
        );

        self.update_cached_destination();

        if let Ok(controlled_pawn) = me.k2_get_pawn().checked() {
            let world_direction = (self.cached_destination()
                - controlled_pawn.k2_get_actor_location().to_dvec3())
            .normalize_or_zero();

            controlled_pawn.add_movement_input(
                &FVector::from_dvec3(world_direction),
                Some(1.0),
                Some(false),
            );
        }
    }

    #[ufunction]
    fn on_set_destination_released(&mut self) {
        if self.follow_time() <= self.short_press_threshold() {
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
    fn setup_input(&self) -> RustealResult<bool> {
        let me: UObjectRef<PlayerController> = self.as_ref();

        if !me.checked()?.is_local_player_controller() {
            return Ok(false);
        }

        enhanced_input_subsystem(me)?
            .checked()?
            .add_mapping_context(self.default_mapping_context(), 0, &OwnedStruct::new());

        let click = self.set_destination_click_action();
        bind_action(&me, click, ETriggerEvent::Started, "OnInputStarted")?;

        bind_action(
            &me,
            click,
            ETriggerEvent::Triggered,
            "OnSetDestinationTriggered",
        )?;

        bind_action(
            &me,
            click,
            ETriggerEvent::Completed,
            "OnSetDestinationReleased",
        )?;

        bind_action(
            &me,
            click,
            ETriggerEvent::Canceled,
            "OnSetDestinationReleased",
        )?;

        let touch = self.set_destination_touch_action();
        bind_action(&me, touch, ETriggerEvent::Started, "OnInputStarted")?;
        bind_action(&me, touch, ETriggerEvent::Triggered, "OnTouchTriggered")?;
        bind_action(&me, touch, ETriggerEvent::Completed, "OnTouchReleased")?;
        bind_action(&me, touch, ETriggerEvent::Canceled, "OnTouchReleased")?;

        Ok(true)
    }

    fn update_cached_destination(&mut self) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        let (hit_successful, hit) = if self.is_touch() {
            me.get_hit_result_under_finger_by_channel(
                ETouchIndex::Touch1,
                ETraceTypeQuery::TraceTypeQuery1,
                true,
            )
        } else {
            me.get_hit_result_under_cursor_by_channel(ETraceTypeQuery::TraceTypeQuery1, true)
        };

        if hit_successful {
            self.set_cached_destination(hit.as_ref().get_location().to_dvec3());
        }
    }
}
