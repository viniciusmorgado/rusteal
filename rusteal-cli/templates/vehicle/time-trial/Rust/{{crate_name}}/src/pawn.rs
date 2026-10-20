use bindings::chaos_vehicles::{
    ChaosVehicleMovementComponentExt, ChaosWheeledVehicleMovementComponent, WheeledVehiclePawn,
    WheeledVehiclePawnExt,
};
use bindings::engine::{
    ActorComponentExt, CameraComponent, KismetMathLibrary, KismetSystemLibrary,
    NavMovementComponentExt, PawnExt, PrimitiveComponentExt, SceneComponentExt, SpringArmComponent,
    SpringArmComponentExt,
};
use bindings::enhanced_input::{ETriggerEvent, FInputActionValue, InputAction};
use bindings::prelude::*;
use glam::DVec3;
use rusteal_runtime::runtime::{FName, Rotator, RustealResult, UObjectRef, UStructRef};
use rusteal_runtime::{uclass, uclass_impl};

use crate::model;

#[uclass(parent = WheeledVehiclePawn)]
pub struct VehicleAdvPawn {
    #[component(attach = "mesh", name = "Front Spring Arm")]
    front_spring_arm: SpringArmComponent,

    #[component(attach = "front_spring_arm", name = "Front Camera")]
    front_camera: CameraComponent,

    #[component(attach = "mesh", name = "Back Spring Arm")]
    back_spring_arm: SpringArmComponent,

    #[component(attach = "back_spring_arm", name = "Back Camera")]
    back_camera: CameraComponent,

    #[uproperty(EditAnywhere, category = "Input")]
    steering_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    throttle_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    brake_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    handbrake_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    look_around_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    toggle_camera_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    reset_vehicle_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Flip Check", default = 3.0)]
    flip_check_time: f32,

    #[uproperty(EditAnywhere, category = "Flip Check", default = -0.2)]
    flip_check_min_dot: f32,

    front_camera_active: bool,
    previous_flip_check: bool,
}

#[uclass_impl]
impl VehicleAdvPawn {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let front_arm = self.front_spring_arm()?.checked()?;
        front_arm.set_target_arm_length(0.0);
        front_arm.set_do_collision_test(false);
        front_arm.set_enable_camera_rotation_lag(true);
        front_arm.set_camera_rotation_lag_speed(15.0);

        front_arm.k2_set_relative_location(
            &FVector::from_dvec3(DVec3::new(30.0, 0.0, 120.0)),
            false,
            false,
        );

        self.front_camera()?.checked()?.set_auto_activate(false);

        let back_arm = self.back_spring_arm()?.checked()?;
        back_arm.set_target_arm_length(650.0);
        back_arm.set_socket_offset(&FVector::from_dvec3(DVec3::new(0.0, 0.0, 150.0)));
        back_arm.set_do_collision_test(false);
        back_arm.set_inherit_pitch(false);
        back_arm.set_inherit_roll(false);
        back_arm.set_enable_camera_rotation_lag(true);
        back_arm.set_camera_rotation_lag_speed(2.0);
        back_arm.set_camera_lag_max_distance(50.0);

        let mesh = self.as_ref().checked()?.get_mesh().checked()?;
        mesh.set_simulate_physics(true);
        mesh.set_collision_profile_name(FName::new("Vehicle").handle(), Some(true));

        Ok(())
    }

    #[ufunction(Override)]
    pub(crate) fn receive_restarted(&mut self) {
        if let Err(e) = self.bind_input() {
            log::warn!("[Vehicle] failed to bind the input: {e}");
        }
    }

    #[ufunction(Override)]
    pub(crate) fn receive_begin_play(&mut self) {
        KismetSystemLibrary::k2_set_timer(
            self.as_ref().upcast_to(),
            "FlippedCheck",
            self.flip_check_time(),
            true,
            None,
            None,
            None,
        );
    }

    #[ufunction(Override)]
    pub(crate) fn receive_end_play(&mut self, _end_play_reason: u8) {
        KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), "FlippedCheck");
    }

    #[ufunction(Override)]
    pub(crate) fn receive_tick(&mut self, delta_seconds: f32) {
        if let Err(e) = self.update(delta_seconds) {
            log::warn!("[Vehicle] update failed: {e}");
        }
    }

    #[ufunction]
    fn steering(&mut self, value: UStructRef<FInputActionValue>) {
        self.do_steering(value.axis1d() as f32);
    }

    #[ufunction]
    fn throttle(&mut self, value: UStructRef<FInputActionValue>) {
        self.do_throttle(value.axis1d() as f32);
    }

    #[ufunction]
    fn brake(&mut self, value: UStructRef<FInputActionValue>) {
        self.do_brake(value.axis1d() as f32);
    }

    #[ufunction]
    fn start_brake(&mut self) {
        self.do_brake_start();
    }

    #[ufunction]
    fn stop_brake(&mut self) {
        self.do_brake_stop();
    }

    #[ufunction]
    fn start_handbrake(&mut self) {
        self.do_handbrake_start();
    }

    #[ufunction]
    fn stop_handbrake(&mut self) {
        self.do_handbrake_stop();
    }

    #[ufunction]
    fn look_around(&mut self, value: UStructRef<FInputActionValue>) {
        self.do_look_around(value.axis1d() as f32);
    }

    #[ufunction]
    fn toggle_camera(&mut self) {
        self.do_toggle_camera();
    }

    #[ufunction]
    fn reset_vehicle(&mut self) {
        self.do_reset_vehicle();
    }

    #[ufunction(BlueprintCallable)]
    pub(crate) fn do_steering(&mut self, steering_value: f32) {
        if let Ok(movement) = self.movement() {
            movement.set_steering_input(steering_value);
        }
    }

    #[ufunction(BlueprintCallable)]
    pub(crate) fn do_throttle(&mut self, throttle_value: f32) {
        if let Ok(movement) = self.movement() {
            movement.set_throttle_input(throttle_value);
            movement.set_brake_input(0.0);
        }
    }

    #[ufunction(BlueprintCallable)]
    pub(crate) fn do_brake(&mut self, brake_value: f32) {
        if let Ok(movement) = self.movement() {
            movement.set_brake_input(brake_value);
            movement.set_throttle_input(0.0);
        }
    }

    #[ufunction(BlueprintCallable)]
    pub(crate) fn do_brake_start(&mut self) {
        self.brake_lights(true);
    }

    #[ufunction(BlueprintCallable)]
    pub(crate) fn do_brake_stop(&mut self) {
        self.brake_lights(false);

        if let Ok(movement) = self.movement() {
            movement.set_brake_input(0.0);
        }
    }

    #[ufunction(BlueprintCallable)]
    pub(crate) fn do_handbrake_start(&mut self) {
        if let Ok(movement) = self.movement() {
            movement.set_handbrake_input(true);
        }

        self.brake_lights(true);
    }

    #[ufunction(BlueprintCallable)]
    pub(crate) fn do_handbrake_stop(&mut self) {
        if let Ok(movement) = self.movement() {
            movement.set_handbrake_input(false);
        }

        self.brake_lights(false);
    }

    #[ufunction(BlueprintCallable)]
    pub(crate) fn do_look_around(&mut self, yaw_delta: f32) {
        if let Ok(arm) = self.back_spring_arm().and_then(|arm| arm.checked()) {
            arm.k2_add_local_rotation(
                &FRotator::from_rotator(Rotator::new(0.0, f64::from(yaw_delta), 0.0)),
                false,
                false,
            );
        }
    }

    #[ufunction(BlueprintCallable)]
    pub(crate) fn do_toggle_camera(&mut self) {
        let front_active = !self.front_camera_active();
        self.set_front_camera_active(front_active);

        if let Ok(front) = self.front_camera().and_then(|camera| camera.checked()) {
            front.set_active(front_active, None);
        }

        if let Ok(back) = self.back_camera().and_then(|camera| camera.checked()) {
            back.set_active(!front_active, None);
        }
    }

    #[ufunction(BlueprintCallable)]
    pub(crate) fn do_reset_vehicle(&mut self) {
        if let Err(e) = self.reset() {
            log::warn!("[Vehicle] reset failed: {e}");
        }
    }

    #[ufunction(BlueprintImplementableEvent)]
    fn brake_lights(&self, b_braking: bool) {}

    #[ufunction]
    fn flipped_check(&mut self) {
        let Ok(mesh) = self
            .as_ref()
            .checked()
            .and_then(|me| me.get_mesh().checked())
        else {
            return;
        };

        let up_dot = DVec3::Z.dot(mesh.get_up_vector().to_dvec3()) as f32;
        let check = model::flip_check(
            self.previous_flip_check(),
            up_dot,
            self.flip_check_min_dot(),
        );

        if check.reset {
            self.do_reset_vehicle();
        }

        self.set_previous_flip_check(check.flipped);
    }
}

impl VehicleAdvPawn {
    pub fn movement(
        &self,
    ) -> RustealResult<rusteal_runtime::runtime::Checked<ChaosWheeledVehicleMovementComponent>>
    {
        self.as_ref()
            .checked()?
            .get_vehicle_movement_component()
            .cast::<ChaosWheeledVehicleMovementComponent>()?
            .checked()
    }

    fn bind_input(&self) -> RustealResult<()> {
        let me = self.as_ref();
        let pawn = me.checked()?;

        if !pawn.is_player_controlled() || !pawn.is_locally_controlled() {
            return Ok(());
        }

        bind_action_by_name(
            &me,
            self.steering_action(),
            ETriggerEvent::Triggered,
            "Steering",
        )?;
        bind_action_by_name(
            &me,
            self.steering_action(),
            ETriggerEvent::Completed,
            "Steering",
        )?;
        bind_action_by_name(
            &me,
            self.throttle_action(),
            ETriggerEvent::Triggered,
            "Throttle",
        )?;
        bind_action_by_name(
            &me,
            self.throttle_action(),
            ETriggerEvent::Completed,
            "Throttle",
        )?;
        bind_action_by_name(&me, self.brake_action(), ETriggerEvent::Triggered, "Brake")?;
        bind_action_by_name(
            &me,
            self.brake_action(),
            ETriggerEvent::Started,
            "StartBrake",
        )?;
        bind_action_by_name(
            &me,
            self.brake_action(),
            ETriggerEvent::Completed,
            "StopBrake",
        )?;
        bind_action_by_name(
            &me,
            self.handbrake_action(),
            ETriggerEvent::Started,
            "StartHandbrake",
        )?;
        bind_action_by_name(
            &me,
            self.handbrake_action(),
            ETriggerEvent::Completed,
            "StopHandbrake",
        )?;
        bind_action_by_name(
            &me,
            self.look_around_action(),
            ETriggerEvent::Triggered,
            "LookAround",
        )?;
        bind_action_by_name(
            &me,
            self.toggle_camera_action(),
            ETriggerEvent::Triggered,
            "ToggleCamera",
        )?;
        bind_action_by_name(
            &me,
            self.reset_vehicle_action(),
            ETriggerEvent::Triggered,
            "ResetVehicle",
        )?;

        Ok(())
    }

    fn update(&self, delta_seconds: f32) -> RustealResult<()> {
        let me = self.as_ref().checked()?;
        let on_ground = self.movement()?.is_moving_on_ground();

        me.get_mesh()
            .checked()?
            .set_angular_damping(model::angular_damping(on_ground));

        let arm = self.back_spring_arm()?.checked()?;
        let yaw = arm.get_relative_rotation().to_rotator().yaw;

        let yaw = model::f_interp_to(
            yaw,
            0.0,
            f64::from(delta_seconds),
            model::CAMERA_RECENTER_SPEED,
        );

        arm.k2_set_relative_rotation(
            &FRotator::from_rotator(Rotator::new(0.0, yaw, 0.0)),
            false,
            false,
        );

        Ok(())
    }

    fn reset(&self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;
        let location = me.k2_get_actor_location().to_dvec3() + DVec3::Z * model::RESET_HEIGHT;
        let yaw = me.k2_get_actor_rotation().to_rotator().yaw;

        let transform = KismetMathLibrary::make_transform(
            &FVector::from_dvec3(location),
            &FRotator::from_rotator(Rotator::new(0.0, yaw, 0.0)),
            &FVector::from_dvec3(DVec3::ONE),
        );

        me.k2_set_actor_transform(&transform, false, true);

        let mesh = me.get_mesh().checked()?;
        let zero = FVector::from_dvec3(DVec3::ZERO);
        mesh.set_physics_angular_velocity_in_degrees(&zero, None, None);
        mesh.set_physics_linear_velocity(&zero, None, None);

        Ok(())
    }
}
