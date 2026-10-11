use bindings::chaos_vehicles::{
    ChaosVehicleMovementComponentExt, ChaosVehicleWheel, ChaosVehicleWheelExt,
    ChaosWheeledVehicleMovementComponentExt, ESteeringType, ESweepShape, EVehicleDifferential,
};
use bindings::engine::{PrimitiveComponentExt, SceneComponentExt, StaticMeshComponent};
use bindings::prelude::*;
use glam::DVec3;
use rusteal_runtime::runtime::{FName, Rotator, RustealResult, SubclassOf};
use rusteal_runtime::{uclass, uclass_impl};

use crate::pawn::VehicleAdvPawn;
use crate::setup::{self, EngineSetup, WheelSetup};
use crate::wheels::{VehicleAdvWheelFront, VehicleAdvWheelRear};

#[uclass(parent = VehicleAdvPawn)]
pub struct VehicleAdvOffroadCar {
    #[component(attach = "mesh", name = "Chassis")]
    chassis: StaticMeshComponent,

    #[component(attach = "mesh", socket = "VisWheel_FL", name = "Tire Front Left")]
    tire_front_left: StaticMeshComponent,

    #[component(attach = "mesh", socket = "VisWheel_FR", name = "Tire Front Right")]
    tire_front_right: StaticMeshComponent,

    #[component(attach = "mesh", socket = "VisWheel_BL", name = "Tire Rear Left")]
    tire_rear_left: StaticMeshComponent,

    #[component(attach = "mesh", socket = "VisWheel_BR", name = "Tire Rear Right")]
    tire_rear_right: StaticMeshComponent,
}

#[uclass_impl]
impl VehicleAdvOffroadCar {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let no_collision = FName::new("NoCollision");
        let turned = FRotator::from_rotator(Rotator::new(0.0, 180.0, 0.0));

        for tire in [
            self.tire_front_left()?,
            self.tire_front_right()?,
            self.tire_rear_left()?,
            self.tire_rear_right()?,
        ] {
            tire.checked()?
                .set_collision_profile_name(no_collision.handle(), Some(true));
        }

        for tire in [self.tire_front_right()?, self.tire_rear_right()?] {
            tire.checked()?.k2_set_relative_rotation(&turned, false, false);
        }

        let pawn = VehicleAdvPawn::from_obj(self.as_ref())?;

        pawn.front_spring_arm()?.checked()?.k2_set_relative_location(
            &FVector::from_dvec3(DVec3::new(-5.0, -30.0, 135.0)),
            false,
            false,
        );

        pawn.back_spring_arm()?.checked()?.k2_set_relative_location(
            &FVector::from_dvec3(DVec3::new(0.0, 0.0, 75.0)),
            false,
            false,
        );

        let movement = pawn.movement()?;
        movement.set_chassis_height(160.0);
        movement.set_drag_coefficient(0.1);
        movement.set_downforce_coefficient(0.1);
        movement.set_center_of_mass_override(&FVector::from_dvec3(DVec3::new(0.0, 0.0, 75.0)));
        movement.set_enable_center_of_mass_override(true);
        movement.set_legacy_wheel_friction_position(false);

        let front = SubclassOf::<VehicleAdvOffroadWheelFront>::base().upcast_to::<ChaosVehicleWheel>();
        let rear = SubclassOf::<VehicleAdvOffroadWheelRear>::base().upcast_to::<ChaosVehicleWheel>();

        setup::set_wheels(
            &movement,
            &[
                WheelSetup::new(front, "PhysWheel_FL"),
                WheelSetup::new(front, "PhysWheel_FR"),
                WheelSetup::new(rear, "PhysWheel_BL"),
                WheelSetup::new(rear, "PhysWheel_BR"),
            ],
        )?;

        setup::set_engine(
            &movement,
            &EngineSetup {
                max_torque: 600.0,
                max_rpm: 5000.0,
                idle_rpm: 1200.0,
                brake_effect: 0.05,
                rev_up_moi: 5.0,
                rev_down_rate: 600.0,
            },
        )?;

        setup::set_differential(&movement, EVehicleDifferential::AllWheelDrive, 0.5)?;
        setup::set_steering(&movement, ESteeringType::AngleRatio, 0.7)?;

        Ok(())
    }
}

#[uclass(parent = VehicleAdvWheelFront)]
pub struct VehicleAdvOffroadWheelFront {}

#[uclass_impl]
impl VehicleAdvOffroadWheelFront {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let wheel = self.as_ref().checked()?;
        wheel.set_wheel_radius(50.0);
        wheel.set_cornering_stiffness(750.0);
        wheel.set_friction_force_multiplier(4.0);
        wheel.set_affected_by_engine(true);
        wheel.set_suspension_max_raise(20.0);
        wheel.set_suspension_max_drop(20.0);
        wheel.set_wheel_load_ratio(1.0);
        wheel.set_spring_rate(100.0);
        wheel.set_spring_preload(100.0);
        wheel.set_sweep_shape(ESweepShape::Shapecast);
        wheel.set_max_brake_torque(3000.0);
        wheel.set_max_hand_brake_torque(6000.0);
        Ok(())
    }
}

#[uclass(parent = VehicleAdvWheelRear)]
pub struct VehicleAdvOffroadWheelRear {}

#[uclass_impl]
impl VehicleAdvOffroadWheelRear {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let wheel = self.as_ref().checked()?;
        wheel.set_wheel_radius(50.0);
        wheel.set_cornering_stiffness(750.0);
        wheel.set_friction_force_multiplier(4.0);
        wheel.set_suspension_max_raise(20.0);
        wheel.set_suspension_max_drop(20.0);
        wheel.set_wheel_load_ratio(1.0);
        wheel.set_spring_rate(100.0);
        wheel.set_spring_preload(100.0);
        wheel.set_sweep_shape(ESweepShape::Shapecast);
        wheel.set_max_brake_torque(3000.0);
        wheel.set_max_hand_brake_torque(6000.0);
        Ok(())
    }
}
