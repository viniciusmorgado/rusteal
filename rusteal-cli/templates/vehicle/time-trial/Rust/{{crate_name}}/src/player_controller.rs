use bindings::chaos_vehicles::ChaosVehicleMovementComponentExt;
use bindings::engine::{ControllerExt, GameplayStatics, Pawn, PlayerController, PlayerStart};
use bindings::enhanced_input::{EnhancedInputLocalPlayerSubsystemExt, InputMappingContext};
use bindings::prelude::*;
use bindings::umg::{UserWidget, UserWidgetExt};
use rusteal_runtime::runtime::input::should_display_touch_interface;
use rusteal_runtime::runtime::{
    LOG_ERROR, LOG_WARNING, OwnedStruct, RustealResult, SubclassOf, UObjectRef, UeArray, ulog,
};
use rusteal_runtime::{uclass, uclass_impl};

use crate::pawn::VehicleAdvPawn;
use crate::ui::VehicleAdvUI;

#[uclass(parent = PlayerController, config = "Game")]
pub struct VehicleAdvPlayerController {
    #[uproperty(EditAnywhere, category = "Input|Input Mappings")]
    default_mapping_contexts: UeArray<UObjectRef<InputMappingContext>>,

    #[uproperty(EditAnywhere, category = "Input|Input Mappings")]
    mobile_excluded_mapping_contexts: UeArray<UObjectRef<InputMappingContext>>,

    #[uproperty(EditAnywhere, category = "Input|Touch Controls")]
    mobile_controls_widget_class: SubclassOf<UserWidget>,

    #[uproperty]
    mobile_controls_widget: UObjectRef<UserWidget>,

    #[uproperty(EditAnywhere, Config, category = "Input|Touch Controls", default = false)]
    b_force_touch_controls: bool,

    #[uproperty(EditAnywhere, category = "Input|Steering Wheel Controls", default = false)]
    b_use_steering_wheel_controls: bool,

    #[uproperty(EditAnywhere, category = "Input|Steering Wheel Controls")]
    steering_wheel_input_mapping_context: UObjectRef<InputMappingContext>,

    #[uproperty(EditAnywhere, category = "Vehicle|Respawn")]
    vehicle_pawn_class: SubclassOf<VehicleAdvPawn>,

    #[uproperty]
    vehicle_pawn: UObjectRef<VehicleAdvPawn>,

    #[uproperty(EditAnywhere, name = "VehicleUIClass", category = "Vehicle|UI")]
    vehicle_ui_class: SubclassOf<VehicleAdvUI>,

    #[uproperty(name = "VehicleUI")]
    vehicle_ui: UObjectRef<VehicleAdvUI>,
}

#[uclass_impl]
impl VehicleAdvPlayerController {
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let Ok(me) = self.as_ref().checked() {
            me.set_attach_to_pawn(true);
        }

        if let Err(e) = self.setup_input() {
            ulog!(LOG_WARNING, "[Vehicle] input setup failed: {e}");
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
                if let Ok(mut me) = VehicleAdvPlayerController::from_obj(controller)
                    && let Err(e) = me.respawn()
                {
                    ulog!(LOG_WARNING, "[Vehicle] respawn failed: {e}");
                }
            })
        });

        match bound {
            Ok(binding) => binding.detach(),
            Err(e) => ulog!(LOG_WARNING, "[Vehicle] cannot watch the vehicle: {e}"),
        }
    }
}

impl VehicleAdvPlayerController {
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
                Err(_) => ulog!(LOG_ERROR, "Could not spawn mobile controls widget."),
            }
        }

        match create_widget_of_class(&me, self.vehicle_ui_class()) {
            Ok(widget) => {
                widget.upcast_to::<UserWidget>().checked()?.add_to_player_screen(Some(0));
                self.set_vehicle_ui(widget);
            }
            Err(_) => ulog!(LOG_ERROR, "Could not spawn vehicle UI widget."),
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
        let vehicle = me.get_world()?.spawn_actor_of_class(self.vehicle_pawn_class(), &transform)?;
        me.checked()?.possess(vehicle.upcast_to());

        Ok(())
    }

    fn should_use_touch_controls(&self) -> bool {
        should_display_touch_interface() || self.b_force_touch_controls()
    }
}
