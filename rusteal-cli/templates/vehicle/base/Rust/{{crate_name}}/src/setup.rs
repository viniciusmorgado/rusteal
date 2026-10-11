use bindings::chaos_vehicles::{
    ChaosVehicleWheel, ChaosWheeledVehicleMovementComponent,
    ChaosWheeledVehicleMovementComponentExt, ESteeringType, EVehicleDifferential, FChaosWheelSetup,
    FChaosWheelSetupExt, FVehicleDifferentialConfigExt, FVehicleEngineConfigExt,
    FVehicleSteeringConfigExt, FVehicleTransmissionConfigExt,
};
use bindings::prelude::*;
use glam::DVec3;
use rusteal_runtime::runtime::{Checked, FName, OwnedStruct, RustealResult, SubclassOf};

pub struct WheelSetup {
    pub wheel_class: SubclassOf<ChaosVehicleWheel>,
    pub bone_name: &'static str,
}

impl WheelSetup {
    pub fn new(wheel_class: SubclassOf<ChaosVehicleWheel>, bone_name: &'static str) -> Self {
        WheelSetup {
            wheel_class,
            bone_name,
        }
    }
}

pub struct EngineSetup {
    pub max_torque: f32,
    pub max_rpm: f32,
    pub idle_rpm: f32,
    pub brake_effect: f32,
    pub rev_up_moi: f32,
    pub rev_down_rate: f32,
}

pub struct TransmissionSetup {
    pub automatic_gears: bool,
    pub auto_reverse: bool,
    pub final_ratio: f32,
    pub change_up_rpm: f32,
    pub change_down_rpm: f32,
    pub gear_change_time: f32,
    pub efficiency: f32,
    pub forward_gear_ratios: &'static [f32],
    pub reverse_gear_ratios: &'static [f32],
}

pub fn set_wheels(
    movement: &Checked<ChaosWheeledVehicleMovementComponent>,
    wheels: &[WheelSetup],
) -> RustealResult<()> {
    let setups = movement.wheel_setups();
    setups.clear()?;

    for wheel in wheels {
        let setup = OwnedStruct::<FChaosWheelSetup>::new();
        let fields = setup.as_ref();
        fields.set_wheel_class(wheel.wheel_class);
        fields.set_bone_name(FName::new(wheel.bone_name).handle());
        fields.set_additional_offset(&FVector::from_dvec3(DVec3::ZERO));
        setups.push(&setup)?;
    }

    Ok(())
}

pub fn set_engine(
    movement: &Checked<ChaosWheeledVehicleMovementComponent>,
    engine: &EngineSetup,
) -> RustealResult<()> {
    let config = movement.get_engine_setup();
    let fields = config.as_ref();
    fields.set_max_torque(engine.max_torque);
    fields.set_max_rpm(engine.max_rpm);
    fields.set_engine_idle_rpm(engine.idle_rpm);
    fields.set_engine_brake_effect(engine.brake_effect);
    fields.set_engine_rev_up_moi(engine.rev_up_moi);
    fields.set_engine_rev_down_rate(engine.rev_down_rate);
    movement.set_engine_setup(&config);

    Ok(())
}

pub fn set_transmission(
    movement: &Checked<ChaosWheeledVehicleMovementComponent>,
    transmission: &TransmissionSetup,
) -> RustealResult<()> {
    let config = movement.get_transmission_setup();
    let fields = config.as_ref();
    fields.set_use_automatic_gears(transmission.automatic_gears);
    fields.set_use_auto_reverse(transmission.auto_reverse);
    fields.set_final_ratio(transmission.final_ratio);
    fields.set_change_up_rpm(transmission.change_up_rpm);
    fields.set_change_down_rpm(transmission.change_down_rpm);
    fields.set_gear_change_time(transmission.gear_change_time);
    fields.set_transmission_efficiency(transmission.efficiency);
    fields
        .forward_gear_ratios()
        .set_all(transmission.forward_gear_ratios)?;
    fields
        .reverse_gear_ratios()
        .set_all(transmission.reverse_gear_ratios)?;
    movement.set_transmission_setup(&config);

    Ok(())
}

pub fn set_steering(
    movement: &Checked<ChaosWheeledVehicleMovementComponent>,
    steering_type: ESteeringType,
    angle_ratio: f32,
) -> RustealResult<()> {
    let config = movement.get_steering_setup();
    let fields = config.as_ref();
    fields.set_steering_type(steering_type);
    fields.set_angle_ratio(angle_ratio);
    movement.set_steering_setup(&config);

    Ok(())
}

pub fn set_differential(
    movement: &Checked<ChaosWheeledVehicleMovementComponent>,
    differential_type: EVehicleDifferential,
    front_rear_split: f32,
) -> RustealResult<()> {
    let config = movement.get_differential_setup();
    let fields = config.as_ref();
    fields.set_differential_type(differential_type);
    fields.set_front_rear_split(front_rear_split);
    movement.set_differential_setup(&config);

    Ok(())
}
