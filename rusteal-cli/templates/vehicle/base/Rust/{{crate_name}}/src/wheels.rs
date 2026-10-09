use bindings::chaos_vehicles::{ChaosVehicleWheel, ChaosVehicleWheelExt, EAxleType};
use rusteal_runtime::runtime::RustealResult;
use rusteal_runtime::{uclass, uclass_impl};

#[uclass(parent = ChaosVehicleWheel)]
pub struct VehicleAdvWheelFront {}

#[uclass_impl]
impl VehicleAdvWheelFront {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let wheel = self.as_ref().checked()?;
        wheel.set_axle_type(EAxleType::Front);
        wheel.set_affected_by_steering(true);
        wheel.set_max_steer_angle(40.0);
        Ok(())
    }
}

#[uclass(parent = ChaosVehicleWheel)]
pub struct VehicleAdvWheelRear {}

#[uclass_impl]
impl VehicleAdvWheelRear {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let wheel = self.as_ref().checked()?;
        wheel.set_axle_type(EAxleType::Rear);
        wheel.set_affected_by_handbrake(true);
        wheel.set_affected_by_engine(true);
        Ok(())
    }
}
