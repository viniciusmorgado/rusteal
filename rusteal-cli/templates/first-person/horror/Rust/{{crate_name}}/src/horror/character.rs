use std::time::Duration;

use bindings::engine::{
    ActorExt, CharacterExt, CharacterMovementComponentExt, ELightUnits, LightComponentExt,
    LocalLightComponentExt, PawnExt, SceneComponentExt, SpotLightComponent, SpotLightComponentExt,
};
use bindings::enhanced_input::{ETriggerEvent, InputAction};
use bindings::prelude::*;
use rusteal_runtime::runtime::timer::{self, TimerHandle};
use rusteal_runtime::runtime::{Rotator, RustealResult, UObjectRef, method};
use rusteal_runtime::{uclass, uclass_impl};

use super::model::{SprintSettings, SprintState, SprintUpdate};
use super::ui::HorrorUI;
use crate::character::FirstPersonCharacter;

const SPOT_LIGHT_LOCATION: [f64; 3] = [30.0, 17.5, -5.0];
const SPOT_LIGHT_ROTATION: [f64; 3] = [-18.6, -1.3, 5.26];
const SPOT_LIGHT_INTENSITY: f32 = 0.5;
const SPOT_LIGHT_ATTENUATION_RADIUS: f32 = 1050.0;
const SPOT_LIGHT_INNER_CONE_ANGLE: f32 = 18.7;
const SPOT_LIGHT_OUTER_CONE_ANGLE: f32 = 45.24;

#[uclass(parent = FirstPersonCharacter)]
pub struct HorrorCharacter {
    #[component(attach = "first_person_camera_component", name = "SpotLight")]
    spot_light: SpotLightComponent,

    #[uproperty(EditAnywhere, category = "Input")]
    sprint_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Walk", default = 250.0)]
    walk_speed: f32,

    #[uproperty(EditAnywhere, category = "Sprint", default = 0.03333)]
    sprint_fixed_tick_time: f32,

    #[uproperty(EditAnywhere, category = "Sprint", default = 3.0)]
    sprint_time: f32,

    #[uproperty(EditAnywhere, category = "Sprint", default = 600.0)]
    sprint_speed: f32,

    #[uproperty(EditAnywhere, category = "Recovery", default = 150.0)]
    recovering_walk_speed: f32,

    #[uproperty(EditAnywhere, category = "Recovery", default = 0.0)]
    recovery_time: f32,

    #[uproperty]
    sprint_listener: UObjectRef<HorrorUI>,

    sprint: SprintState,

    sprint_timer: Option<TimerHandle>,
}

#[uclass_impl]
impl HorrorCharacter {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let [x, y, z] = SPOT_LIGHT_LOCATION;
        let [pitch, yaw, roll] = SPOT_LIGHT_ROTATION;
        let spot_light = self.spot_light()?.checked()?;

        spot_light.k2_set_relative_location_and_rotation(
            &FVector::from_dvec3(glam::DVec3::new(x, y, z)),
            &FRotator::from_rotator(Rotator::new(pitch, yaw, roll)),
            false,
            false,
        );

        spot_light.set_intensity(SPOT_LIGHT_INTENSITY);
        spot_light.set_intensity_units(ELightUnits::Lumens);
        spot_light.set_attenuation_radius(SPOT_LIGHT_ATTENUATION_RADIUS);
        spot_light.set_inner_cone_angle(SPOT_LIGHT_INNER_CONE_ANGLE);
        spot_light.set_outer_cone_angle(SPOT_LIGHT_OUTER_CONE_ANGLE);

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let Ok(mut parent) = FirstPersonCharacter::from_obj(self.as_ref()) {
            parent.receive_begin_play();
        }

        let (sprint, update) = SprintState::begin(&self.sprint_settings());
        self.set_sprint(sprint);
        self.apply(update);

        let sprint_timer = timer::set_timer(
            &self.as_ref(),
            Duration::from_secs_f32(self.sprint_fixed_tick_time()),
            true,
            method(self, Self::sprint_fixed_tick),
        )
        .ok();

        self.set_sprint_timer(sprint_timer);
    }

    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        if let Some(sprint_timer) = self.sprint_timer() {
            sprint_timer.clear();
        }
    }

    #[ufunction(Override)]
    fn receive_restarted(&mut self) {
        if let Ok(mut parent) = FirstPersonCharacter::from_obj(self.as_ref()) {
            parent.receive_restarted();
        }

        if let Err(e) = self.bind_sprint() {
            log::warn!("[Horror] failed to bind the sprint input: {e}");
        }
    }

    #[ufunction(BlueprintCallable)]
    fn do_start_sprint(&mut self) {
        let mut sprint = self.sprint();
        let update = sprint.start_sprint(&self.sprint_settings());
        self.set_sprint(sprint);
        self.apply(update);
    }

    #[ufunction(BlueprintCallable)]
    fn do_end_sprint(&mut self) {
        let mut sprint = self.sprint();
        let update = sprint.end_sprint(&self.sprint_settings());
        self.set_sprint(sprint);
        self.apply(update);
    }

    #[ufunction]
    fn sprint_fixed_tick(&mut self) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        let speed = me.get_velocity().to_dvec3().length();
        let mut sprint = self.sprint();
        let update = sprint.fixed_tick(speed, &self.sprint_settings());
        self.set_sprint(sprint);
        self.apply(update);
    }
}

impl HorrorCharacter {
    fn bind_sprint(&self) -> RustealResult<()> {
        let me = self.as_ref();
        let pawn = me.checked()?;

        if !pawn.is_player_controlled() || !pawn.is_locally_controlled() {
            return Ok(());
        }

        let mut start_sprint = method(self, Self::do_start_sprint);
        bind_action(
            &me,
            self.sprint_action(),
            ETriggerEvent::Started,
            move |_| start_sprint(),
        )?;

        let mut end_sprint = method(self, Self::do_end_sprint);
        bind_action(
            &me,
            self.sprint_action(),
            ETriggerEvent::Completed,
            move |_| end_sprint(),
        )?;

        Ok(())
    }

    fn sprint_settings(&self) -> SprintSettings {
        SprintSettings {
            walk_speed: self.walk_speed(),
            fixed_tick_time: self.sprint_fixed_tick_time(),
            sprint_time: self.sprint_time(),
            sprint_speed: self.sprint_speed(),
            recovering_walk_speed: self.recovering_walk_speed(),
        }
    }

    fn apply(&self, update: SprintUpdate) {
        if let Some(speed) = update.max_walk_speed
            && let Ok(movement) = self
                .as_ref()
                .checked()
                .and_then(|me| me.get_character_movement().checked())
        {
            movement.set_max_walk_speed(speed);
        }

        let ui = self.sprint_listener();

        if !ui.is_valid() {
            return;
        }

        let Ok(ui) = HorrorUI::from_obj(ui) else {
            return;
        };

        if let Some(sprinting) = update.sprint_state_changed {
            ui.on_sprint_state_changed(sprinting);
        }

        if let Some(percent) = update.meter_percent {
            ui.on_sprint_meter_updated(percent);
        }
    }
}
