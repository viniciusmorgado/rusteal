use bindings::engine::{
    Actor, ActorComponent, ActorExt, EAttachmentRule, FHitResultExt, GameplayStatics,
    KismetMathLibrary, Pawn, PlayerCameraManagerExt, SceneComponent, SceneComponentExt,
    StaticMeshComponent,
};
use bindings::enhanced_input::{
    ETriggerEvent, EnhancedInputLibrary, EnhancedInputLocalPlayerSubsystemExt, InputAction,
    InputMappingContext,
};
use bindings::head_mounted_display::{MotionControllerComponent, MotionControllerComponentExt};
use bindings::niagara::{NiagaraComponent, NiagaraDataInterfaceArrayFunctionLibrary};
use bindings::prelude::*;
use bindings::umg::{
    SlateBlueprintLibrary, WidgetComponent, WidgetComponentExt, WidgetExt,
    WidgetInteractionComponent, WidgetInteractionComponentExt,
};
use glam::{DVec2, DVec3};
use rusteal_runtime::runtime::{FName, OwnedStruct, RustealResult, SubclassOf, UObjectRef, method};
use rusteal_runtime::{uclass, uclass_impl};

use crate::model;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum PointerMode {
    #[default]
    None,
    Laser,
    Cursor,
}

#[uclass(parent = Actor)]
pub struct Menu {
    #[component(root)]
    root: SceneComponent,

    #[component(attach = "root")]
    widget: WidgetComponent,

    #[component(attach = "widget")]
    cursor: StaticMeshComponent,

    #[component(attach = "root")]
    menu_laser: NiagaraComponent,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Menu", default = 10.0)]
    menu_distance_towards_camera: f32,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Menu", default = -8.0)]
    cursor_speed: f32,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Menu", default = false)]
    b_active_menu_hand_right: bool,

    #[uproperty(EditAnywhere, category = "Input")]
    menu_mapping_context: UObjectRef<InputMappingContext>,

    #[uproperty(EditAnywhere, category = "Input")]
    menu_cursor_left_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    menu_cursor_right_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    menu_interact_left_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    menu_interact_right_action: UObjectRef<InputAction>,

    #[uproperty(BlueprintReadOnly, category = "Menu")]
    widget_interaction_ref_left: UObjectRef<WidgetInteractionComponent>,

    #[uproperty(BlueprintReadOnly, category = "Menu")]
    widget_interaction_ref_right: UObjectRef<WidgetInteractionComponent>,

    #[uproperty(BlueprintReadOnly, category = "Menu")]
    motion_controller_aim_ref: UObjectRef<MotionControllerComponent>,

    #[uproperty(BlueprintReadOnly, category = "Menu")]
    widget_relative_scale_original: f32,

    #[uproperty(BlueprintReadOnly, category = "Menu")]
    cursor_location_limit_y: f32,

    #[uproperty(BlueprintReadOnly, category = "Menu")]
    cursor_location_limit_z: f32,

    open_animation: Option<model::Timeline>,
    pointer_mode: PointerMode,
}

#[uclass_impl]
impl Menu {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        self.as_ref().checked()?.set_input_priority(1);
        Ok(())
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let Err(e) = self.open() {
            log::warn!("[VR] the menu did not open: {e}");
        }
    }

    #[ufunction(Override)]
    fn receive_tick(&mut self, delta_seconds: f32) {
        self.animate(f64::from(delta_seconds));

        if self.as_ref().checked().is_err() {
            return;
        }

        if let Err(e) = self.update_pointer() {
            log::warn!("[VR] menu update failed: {e}");
        }
    }

    #[ufunction(BlueprintCallable)]
    pub fn close_menu(&self) {
        let me = self.as_ref();
        let controller = GameplayStatics::get_player_controller(me.upcast_to(), 0);

        if let Ok(subsystem) = enhanced_input_subsystem(controller).and_then(|s| s.checked()) {
            subsystem.remove_mapping_context(self.menu_mapping_context(), &OwnedStruct::new());
        }

        if let Ok(mut menu) = Menu::from_obj(me) {
            let mut timeline = menu
                .open_animation()
                .unwrap_or(model::Timeline::new(model::MENU_OPEN_SECONDS));
            timeline.reverse();
            menu.set_open_animation(Some(timeline));
        }
    }

    #[ufunction(BlueprintPure)]
    pub fn get_widget_interaction_component(
        &self,
        b_right_wic: bool,
    ) -> UObjectRef<WidgetInteractionComponent> {
        if b_right_wic {
            self.widget_interaction_ref_right()
        } else {
            self.widget_interaction_ref_left()
        }
    }

    #[ufunction]
    fn interact_left_pressed(&mut self) {
        press(self.widget_interaction_ref_left(), true);
    }

    #[ufunction]
    fn interact_left_released(&mut self) {
        press(self.widget_interaction_ref_left(), false);
    }

    #[ufunction]
    fn interact_right_pressed(&mut self) {
        press(self.widget_interaction_ref_right(), true);
    }

    #[ufunction]
    fn interact_right_released(&mut self) {
        press(self.widget_interaction_ref_right(), false);
    }
}

impl Menu {
    pub fn spawn_for(
        owner: UObjectRef<Pawn>,
        class: SubclassOf<Menu>,
        right_hand: bool,
    ) -> RustealResult<UObjectRef<Menu>> {
        let world = owner.get_world()?;
        let transform = FTransform::from_transform(rusteal_runtime::runtime::Transform::new(
            glam::DQuat::IDENTITY,
            DVec3::ZERO,
            DVec3::ONE,
        ));

        let params = SpawnParams {
            instigator: Some(owner),
            ..SpawnParams::default()
        };

        let menu = world.spawn_actor_deferred_of_class(class, &transform, params)?;
        Menu::from_obj(menu)?.set_b_active_menu_hand_right(right_hand);
        world.finish_spawning(&menu.upcast_to(), &transform)?;
        Ok(menu)
    }

    fn open(&mut self) -> RustealResult<()> {
        let me = self.as_ref();
        let controller = GameplayStatics::get_player_controller(me.upcast_to(), 0);

        me.checked()?.enable_input(controller);

        let subsystem = enhanced_input_subsystem(controller)?.checked()?;
        subsystem.add_mapping_context(self.menu_mapping_context(), 2, &OwnedStruct::new());

        let bindings = [
            (
                self.menu_interact_left_action(),
                ETriggerEvent::Triggered,
                method(self, Self::interact_left_pressed),
            ),
            (
                self.menu_interact_left_action(),
                ETriggerEvent::Canceled,
                method(self, Self::interact_left_released),
            ),
            (
                self.menu_interact_left_action(),
                ETriggerEvent::Completed,
                method(self, Self::interact_left_released),
            ),
            (
                self.menu_interact_right_action(),
                ETriggerEvent::Triggered,
                method(self, Self::interact_right_pressed),
            ),
            (
                self.menu_interact_right_action(),
                ETriggerEvent::Canceled,
                method(self, Self::interact_right_released),
            ),
            (
                self.menu_interact_right_action(),
                ETriggerEvent::Completed,
                method(self, Self::interact_right_released),
            ),
        ];

        for (action, event, mut callback) in bindings {
            if action.is_valid() {
                bind_action(&me, action, event, move |_| callback())?;
            }
        }

        self.set_widget_interaction_references()?;
        self.set_motion_controller_reference()?;

        let widget = self.widget()?.checked()?;
        self.set_widget_relative_scale_original(widget.get_relative_scale3_d().to_dvec3().x as f32);

        let mut timeline = model::Timeline::new(model::MENU_OPEN_SECONDS);
        timeline.play();
        self.set_open_animation(Some(timeline));
        self.apply_open_alpha(0.0)?;
        Ok(())
    }

    fn set_widget_interaction_references(&mut self) -> RustealResult<()> {
        let pawn = GameplayStatics::get_player_pawn(self.as_ref().upcast_to(), 0).checked()?;

        let components = pawn.k2_get_components_by_class(
            SubclassOf::<WidgetInteractionComponent>::base().upcast_to::<ActorComponent>(),
        );

        if components.is_empty() {
            log::warn!(
                "Warning: No WidgetInteractionComponent was found. Menu interaction will FAIL"
            );
        }

        for component in components {
            let interaction = component.cast::<WidgetInteractionComponent>()?;

            if interaction.checked()?.get_pointer_index() == 0 {
                self.set_widget_interaction_ref_left(interaction);
            } else {
                self.set_widget_interaction_ref_right(interaction);
            }
        }

        Ok(())
    }

    fn set_motion_controller_reference(&mut self) -> RustealResult<()> {
        let pawn = GameplayStatics::get_player_pawn(self.as_ref().upcast_to(), 0).checked()?;
        let wanted = if self.b_active_menu_hand_right() {
            "RightAim"
        } else {
            "LeftAim"
        };

        let components = pawn.k2_get_components_by_class(
            SubclassOf::<MotionControllerComponent>::base().upcast_to::<ActorComponent>(),
        );

        if components.is_empty() {
            log::warn!(
                "Warning: No WidgetInteractionComponent was found. Menu interaction will FAIL"
            );
        }

        for component in components {
            let controller = component.cast::<MotionControllerComponent>()?;

            if FName(controller.checked()?.get_motion_source()).to_string_lossy() == wanted {
                self.set_motion_controller_aim_ref(controller);
            }
        }

        Ok(())
    }

    fn animate(&mut self, delta_seconds: f64) {
        let Some(mut timeline) = self.open_animation() else {
            return;
        };

        if !timeline.playing {
            return;
        }

        let finished = timeline.advance(delta_seconds);
        self.set_open_animation(Some(timeline));

        if let Err(e) = self.apply_open_alpha(timeline.alpha()) {
            log::warn!("[VR] menu animation failed: {e}");
        }

        if finished && let Err(e) = self.animation_finished(timeline.forward) {
            log::warn!("[VR] menu animation failed: {e}");
        }
    }

    fn apply_open_alpha(&self, alpha: f64) -> RustealResult<()> {
        let scale = alpha * f64::from(self.widget_relative_scale_original());
        self.widget()?
            .checked()?
            .set_relative_scale3_d(&FVector::from_dvec3(DVec3::splat(scale)));
        Ok(())
    }

    fn animation_finished(&mut self, forward: bool) -> RustealResult<()> {
        let widget = self.widget()?.checked()?;
        let user_widget = widget.get_user_widget_object();

        if let Ok(user_widget) = user_widget.checked() {
            let size =
                SlateBlueprintLibrary::get_absolute_size(&user_widget.get_paint_space_geometry())
                    .to_dvec2();
            let limits = model::cursor_limits(DVec2::new(size.x, size.y));
            self.set_cursor_location_limit_y(limits.x as f32);
            self.set_cursor_location_limit_z(limits.y as f32);
        }

        if forward {
            return Ok(());
        }

        let active = self.get_widget_interaction_component(self.b_active_menu_hand_right());

        if let Ok(active) = active.checked() {
            active.k2_attach_to_component(
                self.motion_controller_aim_ref().upcast_to(),
                FName::NONE.handle(),
                EAttachmentRule::SnapToTarget,
                EAttachmentRule::SnapToTarget,
                EAttachmentRule::KeepWorld,
                true,
            );
        }

        self.as_ref().checked()?.k2_destroy_actor();
        Ok(())
    }

    fn update_pointer(&mut self) -> RustealResult<()> {
        let me = self.as_ref();
        let aim = self.motion_controller_aim_ref().checked()?;
        let camera = GameplayStatics::get_player_camera_manager(me.upcast_to(), 0).checked()?;
        let camera_location = camera.get_camera_location();
        let aim_location = aim.k2_get_component_location();

        let look = KismetMathLibrary::find_look_at_rotation(&aim_location, &camera_location);
        let forward = KismetMathLibrary::get_forward_vector(&look).to_dvec3();
        let up = KismetMathLibrary::get_up_vector(&look).to_dvec3();
        let distance = f64::from(self.menu_distance_towards_camera());

        let widget = self.widget()?.checked()?;
        widget.k2_set_world_location(
            &FVector::from_dvec3(aim_location.to_dvec3() + forward * distance + up * distance),
            false,
            false,
        );

        let facing = KismetMathLibrary::find_look_at_rotation(
            &widget.k2_get_component_location(),
            &camera_location,
        );
        widget.k2_set_world_rotation(&facing, false, false);

        let right = self.b_active_menu_hand_right();
        let laser_hand = self.get_widget_interaction_component(!right);

        let Ok(laser) = laser_hand.checked() else {
            return Ok(());
        };

        if laser.is_over_hit_test_visible_widget() {
            if self.pointer_mode() != PointerMode::Laser {
                self.set_pointer_mode(PointerMode::Laser);
                self.menu_laser()?
                    .checked()?
                    .set_visibility(true, Some(false));
                self.attach_active_interaction(aim.as_ref().upcast_to(), false)?;
            }

            let impact = laser
                .get_last_hit_result()
                .as_ref()
                .get_impact_point()
                .to_dvec3();
            self.cursor()?.checked()?.k2_set_world_location(
                &FVector::from_dvec3(impact),
                false,
                false,
            );

            let laser_system = self.menu_laser()?;
            let points = FName::new("User.PointArray");
            NiagaraDataInterfaceArrayFunctionLibrary::set_niagara_array_vector_value(
                laser_system,
                points.handle(),
                0,
                &laser.k2_get_component_location(),
                false,
            );
            NiagaraDataInterfaceArrayFunctionLibrary::set_niagara_array_vector_value(
                laser_system,
                points.handle(),
                1,
                &FVector::from_dvec3(impact),
                false,
            );
        } else {
            if self.pointer_mode() != PointerMode::Cursor {
                self.set_pointer_mode(PointerMode::Cursor);
                self.menu_laser()?
                    .checked()?
                    .set_visibility(false, Some(false));
                self.attach_active_interaction(self.cursor()?.upcast_to(), true)?;
            }

            let action = if right {
                self.menu_cursor_right_action()
            } else {
                self.menu_cursor_left_action()
            };
            let input = EnhancedInputLibrary::get_bound_action_value(me.upcast_to(), action)
                .as_ref()
                .axis3d();

            let cursor = self.cursor()?.checked()?;
            let location = cursor.get_relative_location().to_dvec3();
            let speed = f64::from(self.cursor_speed());

            cursor.k2_set_relative_location(
                &FVector::from_dvec3(DVec3::new(
                    0.0,
                    model::cursor_step(
                        location.y,
                        input.x,
                        speed,
                        f64::from(self.cursor_location_limit_y()),
                    ),
                    model::cursor_step(
                        location.z,
                        input.y,
                        speed.abs(),
                        f64::from(self.cursor_location_limit_z()),
                    ),
                )),
                false,
                false,
            );
        }

        Ok(())
    }

    fn attach_active_interaction(
        &self,
        parent: UObjectRef<SceneComponent>,
        offset: bool,
    ) -> RustealResult<()> {
        let active = self
            .get_widget_interaction_component(self.b_active_menu_hand_right())
            .checked()?;

        active.k2_attach_to_component(
            parent,
            FName::NONE.handle(),
            EAttachmentRule::SnapToTarget,
            EAttachmentRule::SnapToTarget,
            EAttachmentRule::KeepWorld,
            true,
        );

        if offset {
            active.k2_set_relative_location(
                &FVector::from_dvec3(DVec3::new(model::CURSOR_WIDGET_OFFSET, 0.0, 0.0)),
                false,
                true,
            );
        }

        Ok(())
    }
}

fn press(interaction: UObjectRef<WidgetInteractionComponent>, pressed: bool) {
    let Ok(interaction) = interaction.checked() else {
        return;
    };

    let key = FKey::named("LeftMouseButton");

    if pressed {
        interaction.press_pointer_key(&key);
    } else {
        interaction.release_pointer_key(&key);
    }
}
