use bindings::augmented_reality::{
    ARBasicLightEstimate, ARBasicLightEstimateExt, ARBlueprintLibrary, ARPlaneGeometry,
    ARPlaneGeometryExt, ARTraceResultLibrary, ARTrackedGeometryExt, EARTextureType,
};
use bindings::engine::{
    Actor, ActorComponentExt, CameraComponent, DirectionalLight, EDrawDebugTrace, EObjectTypeQuery,
    FHitResultExt, GameplayStatics, KismetSystemLibrary, LightComponentExt, LightExt, Pawn,
    PlayerControllerExt, SceneComponent, SkyLight, SkyLightComponentExt, SkyLightExt, Texture,
    Texture2D,
};
use bindings::input_core::ETouchIndex;
use bindings::prelude::*;
use bindings::umg::UserWidgetExt;
use glam::{DVec2, DVec3};
use rusteal_runtime::runtime::{
    LOG_WARNING, LinearColor, OwnedStruct, RustealResult, SubclassOf, UObjectRef, UStructRef,
    UeArray, ulog,
};
use rusteal_runtime::{uclass, uclass_impl};

use crate::enums::EGestureType;
use crate::menus::MainMenu;
use crate::model::{self, Gestures, Touch};
use crate::placeable::Placeable;
use crate::plane::Plane;

#[uclass(parent = Pawn)]
pub struct ARPawn {
    #[component(root)]
    root: SceneComponent,

    #[component(attach = "root")]
    camera: CameraComponent,

    #[uproperty(EditAnywhere, category = "AR")]
    main_menu_class: SubclassOf<MainMenu>,

    #[uproperty(EditAnywhere, category = "AR")]
    placeable_class: SubclassOf<Placeable>,

    #[uproperty(EditAnywhere, category = "AR")]
    plane_class: SubclassOf<Plane>,

    #[uproperty(BlueprintReadOnly, category = "AR")]
    main_menu: UObjectRef<MainMenu>,

    #[uproperty(BlueprintReadOnly, category = "AR")]
    placeable_object: UObjectRef<Placeable>,

    #[uproperty(BlueprintReadOnly, category = "AR")]
    selected_plane: UObjectRef<Plane>,

    #[uproperty(BlueprintReadOnly, name = "BPPlaneCandidate", category = "AR")]
    bp_plane_candidate: UObjectRef<Plane>,

    #[uproperty(BlueprintReadWrite, category = "AR", default = false)]
    scanning_is_done: bool,

    #[uproperty(BlueprintReadOnly, category = "AR", default = false)]
    plane_is_selected: bool,

    #[uproperty(BlueprintReadOnly, category = "AR", default = false)]
    object_is_placed: bool,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "AR", default = false)]
    snapping_transform: bool,

    #[uproperty(
        EditAnywhere,
        BlueprintReadWrite,
        name = "ShowHUDInteraction",
        category = "AR",
        default = true
    )]
    show_hud_interaction: bool,

    #[uproperty(BlueprintReadOnly, category = "AR", default = EGestureType::Translation)]
    current_transformation: EGestureType,

    #[uproperty(BlueprintReadOnly, category = "AR")]
    camera_depth_texture: UObjectRef<Texture>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "AR")]
    camera_depth_fallback: UObjectRef<Texture2D>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "AR")]
    color_array: UeArray<OwnedStruct<FLinearColor>>,

    gestures: Gestures,
    show_plane_timer: f64,
    previous_finger_position: DVec2,
    previous_touch1: Touch,
    previous_touch2: Touch,
}

#[uclass_impl]
impl ARPawn {
    #[udelegate(name = "ARObjectSpawned")]
    pub fn ar_object_spawned(&self) {}

    #[udelegate(name = "BPPlaneIsSelected")]
    pub fn bp_plane_is_selected(&self, bp_plane: UObjectRef<Plane>) {}

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let Err(e) = self.add_main_menu() {
            ulog!(LOG_WARNING, "[AR] no main menu: {e}");
        }
    }

    #[ufunction(Override)]
    fn receive_tick(&mut self, delta_seconds: f32) {
        if let Err(e) = self.update(f64::from(delta_seconds)) {
            ulog!(LOG_WARNING, "[AR] pawn update failed: {e}");
        }
    }

    #[ufunction(BlueprintCallable)]
    pub fn handle_touch_hit(
        &mut self,
        hit_actor: UObjectRef<Actor>,
        location: UStructRef<FVector>,
    ) {
        let location = location.to_owned().to_dvec3();

        if let Err(e) = self.touch_hit(hit_actor, location) {
            ulog!(LOG_WARNING, "[AR] touch failed: {e}");
        }
    }

    #[ufunction(BlueprintCallable)]
    pub fn one_finger_pressed(&mut self, hit_actor: UObjectRef<Actor>) {
        if hit_actor.cast::<Placeable>().is_ok() {
            let mut gestures = self.gestures();
            gestures.one_finger_pressed = true;
            gestures.one_finger_timer = 0.0;
            self.set_gestures(gestures);
        }
    }

    #[ufunction(BlueprintCallable)]
    pub fn one_finger_released(&mut self) {
        let mut gestures = self.gestures();
        gestures.one_finger_pressed = false;
        self.set_gestures(gestures);
    }

    #[ufunction(BlueprintCallable)]
    pub fn two_finger_pressed(&mut self) {
        let location = self.touch(ETouchIndex::Touch1).location;
        let mut gestures = self.gestures();
        gestures.two_finger_pressed = true;
        gestures.two_fingers_timer = 0.0;
        gestures.previous_fingers_distance = self.fingers_distance();
        self.set_gestures(gestures);
        self.set_previous_finger_position(location);
    }

    #[ufunction(BlueprintCallable)]
    pub fn two_finger_released(&mut self) {
        let mut gestures = self.gestures();
        gestures.two_finger_pressed = false;
        self.set_gestures(gestures);
    }

    #[ufunction(BlueprintCallable)]
    pub fn reset_placement(&mut self) {
        self.set_object_is_placed(false);
        self.set_plane_is_selected(false);

        if let Ok(placeable) = self.placeable_object().checked() {
            placeable.k2_destroy_actor();
        }

        if let Ok(plane) = self.selected_plane().checked() {
            plane.k2_destroy_actor();
        }
    }

    #[ufunction(BlueprintCallable)]
    pub fn feed_light_estimate(&self) {
        let estimate =
            ARBlueprintLibrary::get_current_light_estimate().cast::<ARBasicLightEstimate>();

        let Ok(estimate) = estimate.and_then(|e| e.checked()) else {
            return;
        };

        let me = self.as_ref();
        let intensity =
            model::light_intensity(f64::from(estimate.get_ambient_intensity_lumens())) as f32;

        let directional = GameplayStatics::get_all_actors_of_class(
            me.upcast_to(),
            SubclassOf::<DirectionalLight>::base().upcast_to(),
        );

        if let Some(light) = directional
            .first()
            .and_then(|a| a.cast::<DirectionalLight>().ok())
            && let Ok(component) = light
                .checked()
                .and_then(|l| l.get_light_component().checked())
        {
            component.set_intensity(intensity);
            component.set_light_color(&estimate.get_ambient_color(), Some(true));
        }

        let sky = GameplayStatics::get_all_actors_of_class(
            me.upcast_to(),
            SubclassOf::<SkyLight>::base().upcast_to(),
        );

        if let Some(light) = sky.first().and_then(|a| a.cast::<SkyLight>().ok())
            && let Ok(component) = light
                .checked()
                .and_then(|l| l.get_light_component().checked())
        {
            component.set_intensity(intensity);
        }
    }
}

impl ARPawn {
    fn add_main_menu(&mut self) -> RustealResult<()> {
        let controller = GameplayStatics::get_player_controller(self.as_ref().upcast_to(), 0);
        let menu = create_widget_of_class(&controller, self.main_menu_class())?;
        menu.upcast_to::<bindings::umg::UserWidget>()
            .checked()?
            .add_to_viewport(Some(0));
        self.set_main_menu(menu);
        Ok(())
    }

    fn touch(&self, index: ETouchIndex) -> Touch {
        let controller = GameplayStatics::get_player_controller(self.as_ref().upcast_to(), 0);

        match controller.checked() {
            Ok(controller) => {
                let (x, y, pressed) = controller.get_input_touch_state(index);
                Touch {
                    pressed,
                    location: DVec2::new(f64::from(x), f64::from(y)),
                }
            }
            Err(_) => Touch::default(),
        }
    }

    fn fingers_distance(&self) -> f64 {
        self.touch(ETouchIndex::Touch1)
            .location
            .distance(self.touch(ETouchIndex::Touch2).location)
    }

    fn fingers_direction(&self) -> DVec2 {
        model::normal_2d(self.touch(ETouchIndex::Touch1).location - self.previous_finger_position())
    }

    fn pinch(&self) -> f64 {
        let controller = GameplayStatics::get_player_controller(self.as_ref().upcast_to(), 0);

        controller
            .checked()
            .map(|c| f64::from(c.get_input_analog_key_state(&FKey::named("Gesture_Pinch"))))
            .unwrap_or(0.0)
    }

    fn poll_touches(&mut self) -> RustealResult<()> {
        let touch1 = self.touch(ETouchIndex::Touch1);
        let touch2 = self.touch(ETouchIndex::Touch2);
        let previous1 = self.previous_touch1();
        let previous2 = self.previous_touch2();
        self.set_previous_touch1(touch1);
        self.set_previous_touch2(touch2);

        if touch1.pressed && !previous1.pressed {
            let hit = self.world_hit_test(touch1.location);
            let actor = hit.map(|(actor, _)| actor).unwrap_or_default();
            self.one_finger_pressed(actor);

            if let Some((actor, location)) = hit {
                self.touch_hit(actor, location)?;
            }
        } else if !touch1.pressed && previous1.pressed {
            self.one_finger_released();
        }

        if touch2.pressed && !previous2.pressed {
            self.two_finger_pressed();
        } else if !touch2.pressed && previous2.pressed {
            self.two_finger_released();
        }

        Ok(())
    }

    fn world_hit_test(&self, touch_location: DVec2) -> Option<(UObjectRef<Actor>, DVec3)> {
        let me = self.as_ref();
        let controller = GameplayStatics::get_player_controller(me.upcast_to(), 0);
        let (_, origin, direction) = GameplayStatics::deproject_screen_to_world(
            controller,
            &FVector2D::from_dvec2(touch_location),
        );
        let start = origin.to_dvec3();
        let end = start + direction.to_dvec3() * model::HIT_TEST_DISTANCE;

        let (hit, result) = KismetSystemLibrary::line_trace_single_for_objects(
            me.upcast_to(),
            &FVector::from_dvec3(start),
            &FVector::from_dvec3(end),
            &[
                EObjectTypeQuery::ObjectTypeQuery1,
                EObjectTypeQuery::ObjectTypeQuery2,
                EObjectTypeQuery::ObjectTypeQuery3,
                EObjectTypeQuery::ObjectTypeQuery4,
                EObjectTypeQuery::ObjectTypeQuery5,
                EObjectTypeQuery::ObjectTypeQuery6,
            ],
            false,
            &[],
            EDrawDebugTrace::None,
            true,
            &FLinearColor::from_linear_color(LinearColor::new(1.0, 0.0, 0.0, 1.0)),
            &FLinearColor::from_linear_color(LinearColor::new(0.0, 1.0, 0.0, 1.0)),
            None,
        );

        if !hit {
            return None;
        }

        let result = result.as_ref();
        let actor = result.get_component().checked().ok()?.get_owner();
        Some((actor, result.get_location().to_dvec3()))
    }

    fn touch_hit(&mut self, hit_actor: UObjectRef<Actor>, location: DVec3) -> RustealResult<()> {
        if !self.plane_is_selected() {
            let Ok(plane) = Plane::from_obj(hit_actor) else {
                return Ok(());
            };

            if !plane.is_shown() {
                return Ok(());
            }

            self.set_selected_plane(hit_actor.cast::<Plane>()?);
            self.bp_plane_is_selected(self.selected_plane());
            self.set_plane_is_selected(true);
            return Ok(());
        }

        if self.object_is_placed() || hit_actor != self.selected_plane().upcast_to() {
            return Ok(());
        }

        let world = self.as_ref().get_world()?;
        let transform = FTransform::from_transform(
            rusteal_runtime::runtime::Transform::from_translation(location),
        );
        let placeable = world.spawn_actor_of_class(self.placeable_class(), &transform)?;

        Placeable::from_obj(placeable)?.set_bp_plane(self.selected_plane());
        self.set_placeable_object(placeable);
        self.set_object_is_placed(true);

        if let Ok(plane) = Plane::from_obj(self.selected_plane()) {
            plane.hide();
        }

        self.ar_object_spawned();
        Ok(())
    }

    fn update(&mut self, delta: f64) -> RustealResult<()> {
        self.poll_touches()?;
        self.recognise_gestures(delta)?;

        if !self.scanning_is_done() && !self.camera_depth_texture().is_valid() {
            self.set_camera_depth_texture(
                ARBlueprintLibrary::get_ar_texture(EARTextureType::SceneDepthMap).upcast_to(),
            );
        }

        if self.scanning_is_done() && !self.plane_is_selected() {
            let timer = self.show_plane_timer() + delta;

            if timer > model::PLANE_UPDATE_SECONDS {
                self.set_show_plane_timer(0.0);
                self.update_plane_candidate()?;
            } else {
                self.set_show_plane_timer(timer);
            }
        }

        self.feed_light_estimate();
        self.set_previous_finger_position(self.touch(ETouchIndex::Touch1).location);
        Ok(())
    }

    fn recognise_gestures(&mut self, delta: f64) -> RustealResult<()> {
        let any_touch =
            self.touch(ETouchIndex::Touch1).pressed || self.touch(ETouchIndex::Touch2).pressed;
        let placed = self.object_is_placed();
        let mut gestures = self.gestures();

        if gestures.reset(delta, any_touch)
            && let Ok(mut placeable) = Placeable::from_obj(self.placeable_object())
        {
            placeable.reset_visuals();
        }

        gestures.one_finger(delta, placed);
        gestures.two_fingers(
            delta,
            placed,
            self.fingers_distance(),
            self.fingers_direction(),
        );
        self.set_gestures(gestures);
        self.set_current_transformation(gestures.current);

        if !gestures.recognised {
            return Ok(());
        }

        let Ok(mut placeable) = Placeable::from_obj(self.placeable_object()) else {
            return Ok(());
        };

        let touch = self.touch(ETouchIndex::Touch1).location;
        let snapping = self.snapping_transform();
        let hud = self.show_hud_interaction();

        match gestures.current {
            EGestureType::Translation => placeable.set_placeable_position(
                FVector2D::from_dvec2(touch).as_ref(),
                snapping,
                hud,
            ),
            EGestureType::Rotation => placeable.set_placeable_rotation(
                FVector2D::from_dvec2(self.fingers_direction()).as_ref(),
                FVector2D::from_dvec2(touch).as_ref(),
                snapping,
                hud,
            ),
            EGestureType::Scale => {
                placeable.set_placeable_scale(self.pinch() as f32, snapping, hud)
            }
        }

        Ok(())
    }

    fn update_plane_candidate(&mut self) -> RustealResult<()> {
        let controller = GameplayStatics::get_player_controller(self.as_ref().upcast_to(), 0);
        let (width, height) = controller.checked()?.get_viewport_size();
        let center = DVec2::new(f64::from(width), f64::from(height)) * 0.5;

        let results = ARBlueprintLibrary::line_trace_tracked_objects(
            &FVector2D::from_dvec2(center),
            Some(false),
            Some(false),
            Some(true),
            Some(true),
        );

        let Some(first) = results.first() else {
            return Ok(());
        };

        let geometry = ARTraceResultLibrary::get_tracked_geometry(first);

        let Ok(plane_geometry) = geometry.cast::<ARPlaneGeometry>().and_then(|g| g.checked())
        else {
            return Ok(());
        };

        let world = self.as_ref().get_world()?;
        let plane = world.spawn_actor_of_class(
            self.plane_class(),
            &plane_geometry.get_local_to_world_transform(),
        )?;
        let plane_actor = Plane::from_obj(plane)?;
        plane_actor.set_scale(plane_geometry.get_extent().to_dvec3() / model::PLANE_EXTENT_SCALE);

        let depth = if self.camera_depth_texture().is_valid() {
            self.camera_depth_texture()
        } else {
            self.camera_depth_fallback().upcast_to()
        };

        let mut plane_actor = plane_actor;
        plane_actor.initialize_plane(depth, self.plane_color(geometry));

        if let Ok(old) = self.bp_plane_candidate().checked() {
            old.k2_destroy_actor();
        }

        self.set_bp_plane_candidate(plane);
        Ok(())
    }

    fn plane_color(
        &self,
        geometry: UObjectRef<bindings::augmented_reality::ARTrackedGeometry>,
    ) -> LinearColor {
        let index = ARBlueprintLibrary::get_all_geometries()
            .iter()
            .position(|g| *g == geometry);
        let colors = self.color_array().to_vec().unwrap_or_default();

        model::plane_color_index(index)
            .and_then(|i| colors.get(i))
            .map(|c| c.to_linear_color())
            .unwrap_or(LinearColor::new(0.0, 0.0, 0.0, 0.0))
    }
}
