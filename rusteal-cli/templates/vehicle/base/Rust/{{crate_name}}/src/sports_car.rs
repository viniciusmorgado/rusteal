use bindings::chaos_vehicles::{
    ChaosVehicleMovementComponentExt, ChaosVehicleWheel, ChaosVehicleWheelExt,
    ChaosWheeledVehicleMovementComponentExt, ESteeringType,
};
use rusteal_runtime::runtime::{RustealResult, SubclassOf};
use rusteal_runtime::{uclass, uclass_impl};

use crate::pawn::VehicleAdvPawn;
use crate::setup::{self, EngineSetup, TransmissionSetup, WheelSetup};
use crate::wheels::{VehicleAdvWheelFront, VehicleAdvWheelRear};

#[uclass(parent = VehicleAdvPawn)]
pub struct VehicleAdvSportsCar {}

#[uclass_impl]
impl VehicleAdvSportsCar {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let pawn = VehicleAdvPawn::from_obj(self.as_ref())?;
        let movement = pawn.movement()?;

        movement.set_chassis_height(144.0);
        movement.set_drag_coefficient(0.31);
        movement.set_legacy_wheel_friction_position(true);

        let front =
            SubclassOf::<VehicleAdvSportsWheelFront>::base().upcast_to::<ChaosVehicleWheel>();
        let rear = SubclassOf::<VehicleAdvSportsWheelRear>::base().upcast_to::<ChaosVehicleWheel>();

        setup::set_wheels(
            &movement,
            &[
                WheelSetup::new(front, "Phys_Wheel_FL"),
                WheelSetup::new(front, "Phys_Wheel_FR"),
                WheelSetup::new(rear, "Phys_Wheel_BL"),
                WheelSetup::new(rear, "Phys_Wheel_BR"),
            ],
        )?;

        setup::set_engine(
            &movement,
            &EngineSetup {
                max_torque: 750.0,
                max_rpm: 7000.0,
                idle_rpm: 900.0,
                brake_effect: 0.2,
                rev_up_moi: 5.0,
                rev_down_rate: 600.0,
            },
        )?;

        setup::set_transmission(
            &movement,
            &TransmissionSetup {
                automatic_gears: true,
                auto_reverse: true,
                final_ratio: 2.81,
                change_up_rpm: 6000.0,
                change_down_rpm: 2000.0,
                gear_change_time: 0.2,
                efficiency: 0.9,
                forward_gear_ratios: &[4.25, 2.52, 1.66, 1.22, 1.0],
                reverse_gear_ratios: &[4.04],
            },
        )?;

        setup::set_steering(&movement, ESteeringType::Ackermann, 0.7)?;

        Ok(())
    }
}

#[uclass(parent = VehicleAdvWheelFront)]
pub struct VehicleAdvSportsWheelFront {}

#[uclass_impl]
impl VehicleAdvSportsWheelFront {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let wheel = self.as_ref().checked()?;
        wheel.set_wheel_radius(39.0);
        wheel.set_wheel_width(35.0);
        wheel.set_friction_force_multiplier(3.0);
        wheel.set_max_brake_torque(4500.0);
        wheel.set_max_hand_brake_torque(6000.0);
        Ok(())
    }
}

#[uclass(parent = VehicleAdvWheelRear)]
pub struct VehicleAdvSportsWheelRear {}

#[uclass_impl]
impl VehicleAdvSportsWheelRear {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let wheel = self.as_ref().checked()?;
        wheel.set_wheel_radius(40.0);
        wheel.set_wheel_width(40.0);
        wheel.set_friction_force_multiplier(4.0);
        wheel.set_slip_threshold(100.0);
        wheel.set_skid_threshold(100.0);
        wheel.set_max_steer_angle(0.0);
        wheel.set_max_hand_brake_torque(6000.0);
        Ok(())
    }
}
