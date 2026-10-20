use bindings::engine::{
    Actor, ActorExt, CameraComponent, FPostProcessSettingsExt, GameplayStatics,
    KismetSystemLibrary, PlayerCameraManagerExt, PlayerControllerExt, PrimitiveComponent,
    SceneCaptureComponent2D, SceneCaptureComponent2DExt, SceneCaptureComponentExt, SceneComponent,
    SceneComponentExt, StaticMeshComponent, TextRenderComponent, Texture,
};
use bindings::enhanced_input::{
    ETriggerEvent, EnhancedInputLibrary, EnhancedInputLocalPlayerSubsystemExt, InputAction,
    InputMappingContext,
};
use bindings::head_mounted_display::ESpectatorScreenMode;
use bindings::prelude::*;
use bindings::xr_base::HeadMountedDisplayFunctionLibrary;
use glam::DVec3;
use rusteal_runtime::runtime::{OwnedStruct, Rotator, RustealResult, SubclassOf, UObjectRef};
use rusteal_runtime::{uclass, uclass_impl, uenum};

use crate::model;

#[uenum]
pub enum EVRSpectatorMode {
    Fly = 1,
    FirstPerson = 2,
}

#[uclass(parent = Actor)]
pub struct VRSpectator {
    #[component(root)]
    root: SceneComponent,

    #[component(attach = "root", name = "SceneCaptureComponent2D")]
    scene_capture_component2_d: SceneCaptureComponent2D,

    #[component(attach = "scene_capture_component2_d")]
    camera_mesh: StaticMeshComponent,

    #[component(attach = "camera_mesh", name = "TextRender_EditorOnly")]
    text_render_editor_only: TextRenderComponent,

    #[uproperty(
        EditAnywhere,
        BlueprintReadWrite,
        category = "Spectator",
        default = false
    )]
    b_spectator_enabled: bool,

    #[uproperty(EditAnywhere, BlueprintReadWrite, name = "EVRSpectatorMode", category = "Spectator", default = EVRSpectatorMode::Fly)]
    spectator_mode: EVRSpectatorMode,

    #[uproperty(
        EditAnywhere,
        BlueprintReadWrite,
        category = "Spectator",
        default = 2.0
    )]
    camera_movement_interp_speed: f32,

    #[uproperty(
        EditAnywhere,
        BlueprintReadWrite,
        category = "Spectator",
        default = 100.0
    )]
    movement_speed_multiplier: f32,

    #[uproperty(
        EditAnywhere,
        BlueprintReadWrite,
        category = "Spectator",
        default = 2.0
    )]
    rotation_interp_speed: f32,

    #[uproperty(
        EditAnywhere,
        BlueprintReadWrite,
        category = "Spectator",
        default = 3.0
    )]
    fov_interp_speed: f32,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Spectator", default = -3.0)]
    fov_speed_multiplier: f32,

    #[uproperty(
        EditAnywhere,
        BlueprintReadWrite,
        category = "Spectator",
        default = 90.0
    )]
    default_field_of_view: f32,

    #[uproperty(EditAnywhere, category = "Spectator")]
    spectator_texture: UObjectRef<Texture>,

    #[uproperty(EditAnywhere, category = "Input")]
    spectator_mapping_context: UObjectRef<InputMappingContext>,

    #[uproperty(EditAnywhere, category = "Input")]
    toggle_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    fov_reset_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    reset_rotation_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    fade_in_out_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    fov_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    move_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    look_action: UObjectRef<InputAction>,

    #[uproperty(BlueprintReadOnly, category = "Spectator")]
    movement_offset: OwnedStruct<FVector>,

    #[uproperty(BlueprintReadOnly, category = "Spectator")]
    yaw_rotation_delta: f32,

    #[uproperty(BlueprintReadOnly, category = "Spectator")]
    pitch_rotation_delta: f32,

    #[uproperty(BlueprintReadOnly, name = "FOVAngleDelta", category = "Spectator")]
    fov_angle_delta: f32,

    fade: Option<model::Timeline>,
    fade_flip: bool,
}

#[uclass_impl]
impl VRSpectator {
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if !self.disable_if_mobile() {
            return;
        }

        if let Err(e) = self.initialize_input() {
            log::warn!("[VR] spectator input failed: {e}");
        }

        self.initialize_field_of_view();

        if self.b_spectator_enabled() {
            self.enable_vr_spectator();
        }
    }

    #[ufunction(Override)]
    fn receive_destroyed(&mut self) {
        self.disable_vr_spectator();
    }

    #[ufunction(Override)]
    fn receive_tick(&mut self, delta_seconds: f32) {
        if let Err(e) = self.update(f64::from(delta_seconds)) {
            log::warn!("[VR] spectator update failed: {e}");
        }
    }

    #[ufunction]
    fn toggle(&mut self) {
        if self.b_spectator_enabled() {
            self.disable_vr_spectator();
        } else {
            self.enable_vr_spectator();
        }
    }

    #[ufunction]
    fn fov_reset(&mut self) {
        self.set_fov_angle_delta(0.0);
        self.initialize_field_of_view();
    }

    #[ufunction]
    fn reset_rotation(&mut self) {
        if let Ok(capture) = self.scene_capture_component2_d().and_then(|c| c.checked()) {
            capture.k2_set_relative_rotation(
                &FRotator::from_rotator(Rotator::new(0.0, 0.0, 0.0)),
                false,
                false,
            );
        }
    }

    #[ufunction]
    fn fade_in_out(&mut self) {
        let mut fade = self
            .fade()
            .unwrap_or(model::Timeline::new(model::SPECTATOR_FADE_SECONDS));

        if self.fade_flip() {
            fade.reverse();
        } else {
            fade.play();
        }

        self.set_fade_flip(!self.fade_flip());
        self.set_fade(Some(fade));
    }

    #[ufunction(BlueprintCallable)]
    pub fn enable_vr_spectator(&mut self) {
        self.set_vr_spectator_mode(self.spectator_mode());
        self.set_b_spectator_enabled(true);

        let me = self.as_ref();

        if let Ok(actor) = me.checked() {
            actor.set_actor_tick_enabled(true);
        }

        if let Ok(capture) = self.scene_capture_component2_d().and_then(|c| c.checked()) {
            capture.set_capture_every_frame(true);
        }

        if HeadMountedDisplayFunctionLibrary::is_head_mounted_display_enabled() {
            HeadMountedDisplayFunctionLibrary::set_spectator_screen_mode(
                ESpectatorScreenMode::Texture,
            );
            HeadMountedDisplayFunctionLibrary::set_spectator_screen_texture(
                self.spectator_texture(),
            );
        } else {
            let controller = GameplayStatics::get_player_controller(me.upcast_to(), 0);

            if let Ok(controller) = controller.checked() {
                controller.set_view_target_with_blend(
                    me.upcast_to(),
                    Some(0.0),
                    None,
                    Some(0.0),
                    Some(false),
                );
            }

            let red = FLinearColor::from_linear_color(rusteal_runtime::runtime::LinearColor::new(
                1.0, 0.0, 0.0, 1.0,
            ));
            KismetSystemLibrary::print_string(
                me.upcast_to(),
                Some("Spectator Camera Debug Mode Enabled: No HMD!"),
                Some(true),
                Some(true),
                &red,
                Some(6.0),
                None,
            );
        }
    }

    #[ufunction(BlueprintCallable)]
    pub fn disable_vr_spectator(&mut self) {
        self.set_b_spectator_enabled(false);

        if let Ok(actor) = self.as_ref().checked() {
            actor.set_actor_tick_enabled(false);
        }

        HeadMountedDisplayFunctionLibrary::set_spectator_screen_mode(
            ESpectatorScreenMode::SingleEyeCroppedToFill,
        );

        if let Ok(capture) = self.scene_capture_component2_d().and_then(|c| c.checked()) {
            capture.set_capture_every_frame(false);
        }
    }

    #[ufunction(BlueprintCallable)]
    pub fn set_vr_spectator_mode(&mut self, new_vr_spectator_mode: EVRSpectatorMode) {
        self.set_spectator_mode(new_vr_spectator_mode);

        let Ok(capture) = self.scene_capture_component2_d().and_then(|c| c.checked()) else {
            return;
        };

        match new_vr_spectator_mode {
            EVRSpectatorMode::Fly => capture.clear_hidden_components(),
            EVRSpectatorMode::FirstPerson => {
                let pawn = GameplayStatics::get_player_pawn(self.as_ref().upcast_to(), 0);

                let camera = pawn.checked().ok().and_then(|pawn| {
                    pawn.k2_get_components_by_class(
                        SubclassOf::<CameraComponent>::base()
                            .upcast_to::<bindings::engine::ActorComponent>(),
                    )
                    .into_iter()
                    .next()
                });

                let Some(camera) = camera
                    .and_then(|c| c.cast::<SceneComponent>().ok())
                    .and_then(|c| c.checked().ok())
                else {
                    return;
                };

                for child in camera.get_children_components(true) {
                    match child.cast::<PrimitiveComponent>() {
                        Ok(primitive) => capture.hide_component(primitive),
                        Err(_) => log::warn!(
                            "Cast Failed: Camera Child Component was NOT a Primitive Component and won't be hidden"
                        ),
                    }
                }
            }
        }
    }
}

impl VRSpectator {
    fn disable_if_mobile(&self) -> bool {
        if GameplayStatics::get_platform_name() != "Android" {
            return true;
        }

        if let Ok(actor) = self.as_ref().checked() {
            actor.k2_destroy_actor();
        }

        false
    }

    fn initialize_input(&self) -> RustealResult<()> {
        if !HeadMountedDisplayFunctionLibrary::is_head_mounted_display_enabled() {
            return Ok(());
        }

        let me = self.as_ref();
        let controller = GameplayStatics::get_player_controller(me.upcast_to(), 0);
        me.checked()?.enable_input(controller);

        let subsystem = enhanced_input_subsystem(controller)?.checked()?;
        subsystem.add_mapping_context(self.spectator_mapping_context(), 0, &OwnedStruct::new());

        let bindings = [
            (self.toggle_action(), "Toggle"),
            (self.fov_reset_action(), "FovReset"),
            (self.reset_rotation_action(), "ResetRotation"),
            (self.fade_in_out_action(), "FadeInOut"),
        ];

        for (action, function) in bindings {
            if action.is_valid() {
                bind_action_by_name(&me, action, ETriggerEvent::Started, function)?;
            }
        }

        Ok(())
    }

    fn initialize_field_of_view(&self) {
        if let Ok(capture) = self.scene_capture_component2_d().and_then(|c| c.checked()) {
            capture.set_fov_angle(self.default_field_of_view());
        }
    }

    fn input(&self, action: UObjectRef<InputAction>) -> DVec3 {
        EnhancedInputLibrary::get_bound_action_value(self.as_ref().upcast_to(), action)
            .as_ref()
            .axis3d()
    }

    fn update(&mut self, delta_seconds: f64) -> RustealResult<()> {
        self.update_fade(delta_seconds)?;

        let world_delta = GameplayStatics::get_world_delta_seconds(self.as_ref().upcast_to());
        self.handle_field_of_view(world_delta)?;

        match self.spectator_mode() {
            EVRSpectatorMode::Fly => self.handle_spectator_mode_fly(world_delta),
            EVRSpectatorMode::FirstPerson => self.handle_spectator_mode_first_person(),
        }
    }

    fn update_fade(&mut self, delta_seconds: f64) -> RustealResult<()> {
        let Some(mut fade) = self.fade() else {
            return Ok(());
        };

        if !fade.playing {
            return Ok(());
        }

        fade.advance(delta_seconds);
        self.set_fade(Some(fade));

        let gain = model::spectator_fade_gain(fade.alpha());
        let capture = self.scene_capture_component2_d()?.checked()?;
        let settings = capture.get_post_process_settings();
        settings.as_ref().set_override_color_gain(true);
        settings
            .as_ref()
            .set_color_gain(&FVector4::from_dvec4(glam::DVec4::new(
                gain, gain, gain, 1.0,
            )));
        capture.set_post_process_settings(&settings);
        Ok(())
    }

    fn handle_field_of_view(&mut self, world_delta: f64) -> RustealResult<()> {
        let target = self.input(self.fov_action()).x * f64::from(self.fov_speed_multiplier());

        let delta = model::f_interp_to(
            f64::from(self.fov_angle_delta()),
            target,
            world_delta,
            f64::from(self.fov_interp_speed()),
        );
        self.set_fov_angle_delta(delta as f32);

        let capture = self.scene_capture_component2_d()?.checked()?;
        capture
            .set_fov_angle(model::spectator_fov(f64::from(capture.get_fov_angle()), delta) as f32);
        Ok(())
    }

    fn handle_spectator_mode_fly(&mut self, world_delta: f64) -> RustealResult<()> {
        let me = self.as_ref().checked()?;
        let capture = self.scene_capture_component2_d()?.checked()?;
        let look = self.input(self.look_action());
        let rotation_speed = f64::from(self.rotation_interp_speed());

        let yaw = model::f_interp_to(
            f64::from(self.yaw_rotation_delta()),
            look.x,
            world_delta,
            rotation_speed,
        );
        self.set_yaw_rotation_delta(yaw as f32);
        me.k2_add_actor_world_rotation(
            &FRotator::from_rotator(Rotator::new(0.0, yaw, 0.0)),
            false,
            false,
        );

        let pitch = model::f_interp_to(
            f64::from(self.pitch_rotation_delta()),
            look.y,
            world_delta,
            rotation_speed,
        );
        self.set_pitch_rotation_delta(pitch as f32);

        let current_pitch = capture.get_relative_rotation().to_rotator().pitch;

        if !model::spectator_pitch_allowed(current_pitch, pitch) {
            return Ok(());
        }

        capture.k2_add_relative_rotation(
            &FRotator::from_rotator(Rotator::new(pitch, 0.0, 0.0)),
            false,
            false,
        );

        let movement = self.input(self.move_action());
        let target = (capture.get_forward_vector().to_dvec3() * movement.x
            + capture.get_right_vector().to_dvec3() * movement.y
            + DVec3::new(0.0, 0.0, movement.z))
            * (f64::from(self.movement_speed_multiplier()) * world_delta);

        let offset = model::v_interp_to(
            self.movement_offset().to_dvec3(),
            target,
            world_delta,
            f64::from(self.camera_movement_interp_speed()),
        );

        self.set_movement_offset(&FVector::from_dvec3(offset));
        me.k2_add_actor_world_offset(&FVector::from_dvec3(offset), false, false);
        Ok(())
    }

    fn handle_spectator_mode_first_person(&self) -> RustealResult<()> {
        let me = self.as_ref();
        let camera = GameplayStatics::get_player_camera_manager(me.upcast_to(), 0).checked()?;

        me.checked()?.k2_set_actor_location_and_rotation(
            &camera.get_camera_location(),
            &camera.get_camera_rotation(),
            false,
            false,
        );

        Ok(())
    }
}
