use bindings::chaos_vehicles::ChaosVehicleMovementComponentExt;
use bindings::engine::{ControllerExt, GameplayStatics, Pawn, PlayerController, PlayerStart};
use bindings::enhanced_input::{EnhancedInputLocalPlayerSubsystemExt, InputMappingContext};
use bindings::prelude::*;
use bindings::umg::{UserWidget, UserWidgetExt};
use rusteal_runtime::runtime::input::should_display_touch_interface;
use rusteal_runtime::runtime::{
    OwnedStruct, RustealResult, SubclassOf, UObjectRef, UeArray, method,
};
use rusteal_runtime::{uclass, uclass_impl};

use super::game_mode::TimeTrialGameMode;
use super::track_gate::TimeTrialTrackGate;
use super::ui::TimeTrialUI;
use crate::pawn::VehicleAdvPawn;
use crate::ui::VehicleAdvUI;

#[uclass(parent = PlayerController, config = "Game")]
pub struct TimeTrialPlayerController {
    #[uproperty(EditAnywhere, category = "Input|Input Mappings")]
    default_mapping_contexts: UeArray<UObjectRef<InputMappingContext>>,

    #[uproperty(EditAnywhere, category = "Input|Input Mappings")]
    mobile_excluded_mapping_contexts: UeArray<UObjectRef<InputMappingContext>>,

    #[uproperty(EditAnywhere, category = "Input|Touch Controls")]
    mobile_controls_widget_class: SubclassOf<UserWidget>,

    #[uproperty]
    mobile_controls_widget: UObjectRef<UserWidget>,

    #[uproperty(
        EditAnywhere,
        Config,
        category = "Input|Touch Controls",
        default = false
    )]
    b_force_touch_controls: bool,

    #[uproperty(
        EditAnywhere,
        category = "Input|Steering Wheel Controls",
        default = false
    )]
    b_use_steering_wheel_controls: bool,

    #[uproperty(EditAnywhere, category = "Input|Steering Wheel Controls")]
    steering_wheel_input_mapping_context: UObjectRef<InputMappingContext>,

    #[uproperty(EditAnywhere, name = "UIWidgetClass", category = "Time Trial|UI")]
    ui_widget_class: SubclassOf<TimeTrialUI>,

    #[uproperty(name = "UIWidget")]
    ui_widget: UObjectRef<TimeTrialUI>,

    #[uproperty(EditAnywhere, name = "VehicleUIClass", category = "Vehicle|UI")]
    vehicle_ui_class: SubclassOf<VehicleAdvUI>,

    #[uproperty(name = "VehicleUI")]
    vehicle_ui: UObjectRef<VehicleAdvUI>,

    #[uproperty]
    target_gate: UObjectRef<TimeTrialTrackGate>,

    #[uproperty(EditAnywhere, category = "Vehicle|Respawn")]
    vehicle_pawn_class: SubclassOf<VehicleAdvPawn>,

    #[uproperty]
    vehicle_pawn: UObjectRef<VehicleAdvPawn>,

    current_lap: i32,
    race_started: bool,
}

#[uclass_impl]
impl TimeTrialPlayerController {
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let Err(e) = self.setup_input() {
            log::warn!("[TimeTrial] input setup failed: {e}");
        }
    }

    #[ufunction(Override)]
    fn receive_tick(&mut self, _delta_seconds: f32) {
        let (Ok(pawn), Ok(ui)) = (
            VehicleAdvPawn::from_obj(self.vehicle_pawn()),
            VehicleAdvUI::from_obj(self.vehicle_ui()),
        ) else {
            return;
        };

        if let Ok(movement) = pawn.movement() {
            ui.update_speed(movement.get_forward_speed());
            ui.update_gear(movement.get_current_gear());
        }
    }

    #[ufunction(Override)]
    fn receive_possess(&mut self, possessed_pawn: UObjectRef<Pawn>) {
        let Ok(vehicle) = possessed_pawn.cast::<VehicleAdvPawn>() else {
            return;
        };

        self.set_vehicle_pawn(vehicle);

        let controller: UObjectRef<PlayerController> = self.as_ref();

        let bound = possessed_pawn.checked().and_then(|pawn| {
            pawn.on_destroyed().add(move |_destroyed_pawn| {
                if let Ok(mut me) = TimeTrialPlayerController::from_obj(controller)
                    && let Err(e) = me.respawn()
                {
                    log::warn!("[TimeTrial] respawn failed: {e}");
                }
            })
        });

        match bound {
            Ok(binding) => binding.detach(),
            Err(e) => log::warn!("[TimeTrial] cannot watch the vehicle: {e}"),
        }

        if !self.race_started()
            && let Ok(pawn) = possessed_pawn.checked()
        {
            pawn.disable_input(controller);
        }
    }

    #[ufunction]
    fn start_race(&mut self) {
        let game_mode = GameplayStatics::get_game_mode(self.as_ref().upcast_to());

        if let Ok(game_mode) = TimeTrialGameMode::from_obj(game_mode)
            && let Ok(finish_line) = TimeTrialTrackGate::from_obj(game_mode.get_finish_line())
        {
            self.set_target_gate(finish_line.get_next_marker());
        }

        self.set_race_started(true);
        self.set_current_lap(0);
        self.increment_lap_count();

        let me: UObjectRef<PlayerController> = self.as_ref();

        if let Ok(pawn) = me
            .checked()
            .and_then(|controller| controller.k2_get_pawn().checked())
        {
            pawn.enable_input(me);
        }
    }
}

impl TimeTrialPlayerController {
    pub fn increment_lap_count(&mut self) {
        self.set_current_lap(self.current_lap() + 1);

        let now = GameplayStatics::get_time_seconds(self.as_ref().upcast_to()) as f32;

        if let Ok(mut ui) = TimeTrialUI::from_obj(self.ui_widget()) {
            ui.update_lap_count(self.current_lap(), now);
        }
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

        if self.b_use_steering_wheel_controls() {
            contexts.push(self.steering_wheel_input_mapping_context());
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
                Err(_) => log::error!("Could not spawn mobile controls widget."),
            }
        }

        match create_widget_of_class(&me, self.ui_widget_class()) {
            Ok(widget) => {
                widget
                    .upcast_to::<UserWidget>()
                    .checked()?
                    .add_to_player_screen(Some(0));
                self.set_ui_widget(widget);
                TimeTrialUI::from_obj(widget)?
                    .on_race_start_add(method(self, Self::start_race))?
                    .detach();
            }
            Err(_) => log::error!("Could not spawn Time Trial UI widget."),
        }

        match create_widget_of_class(&me, self.vehicle_ui_class()) {
            Ok(widget) => {
                widget
                    .upcast_to::<UserWidget>()
                    .checked()?
                    .add_to_player_screen(Some(0));
                self.set_vehicle_ui(widget);
            }
            Err(_) => log::error!("Could not spawn vehicle UI widget."),
        }

        Ok(())
    }

    fn respawn(&mut self) -> RustealResult<()> {
        let me = self.as_ref();

        let starts = GameplayStatics::get_all_actors_of_class(
            me.upcast_to(),
            SubclassOf::<PlayerStart>::base().upcast_to(),
        );

        let Some(start) = starts.first() else {
            return Ok(());
        };

        let transform = start.checked()?.get_transform();
        let vehicle = me
            .get_world()?
            .spawn_actor_of_class(self.vehicle_pawn_class(), &transform)?;
        me.checked()?.possess(vehicle.upcast_to());

        Ok(())
    }

    fn should_use_touch_controls(&self) -> bool {
        should_display_touch_interface() || self.b_force_touch_controls()
    }
}
