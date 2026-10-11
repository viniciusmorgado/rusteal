use bindings::engine::{
    Actor, ActorComponent, ActorComponentExt, EDrawDebugTrace, EObjectTypeQuery, FHitResultExt,
    GameplayStatics, KismetMaterialLibrary, KismetMathLibrary, KismetSystemLibrary,
    MaterialInstanceDynamic, MaterialInstanceDynamicExt, MaterialInterface, PrimitiveComponentExt,
    SceneComponent, SceneComponentExt, SplineMeshComponent, SplineMeshComponentExt, StaticMesh,
    StaticMeshComponent, StaticMeshComponentExt, Texture, Texture2D,
};
use bindings::prelude::*;
use bindings::umg::{WidgetComponent, WidgetComponentExt};
use glam::{DVec2, DVec3};
use rusteal_runtime::runtime::{
    FName, LOG_WARNING, LinearColor, OwnedStruct, Rotator, RustealResult, UObjectRef, UStructRef,
    ulog,
};
use rusteal_runtime::{uclass, uclass_impl};

use crate::enums::EGestureType;
use crate::menus::InteractionHUD;
use crate::model::{self, Rotation};
use crate::plane::Plane;

#[uclass(parent = ActorComponent)]
pub struct ProductAssetRef {
    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Product")]
    static_mesh: UObjectRef<StaticMesh>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Product", default = 0.5)]
    plate_scale: f32,
}

#[uclass_impl]
impl ProductAssetRef {}

#[uclass(parent = Actor)]
pub struct Placeable {
    #[component(root)]
    scene: SceneComponent,

    #[component(attach = "scene")]
    placeable: SceneComponent,

    #[component(attach = "placeable", name = "SM_Axis")]
    sm_axis: StaticMeshComponent,

    #[component(attach = "placeable", name = "SM_Plate")]
    sm_plate: StaticMeshComponent,

    #[component(attach = "placeable", name = "SM_Shadow")]
    sm_shadow: StaticMeshComponent,

    #[component(attach = "placeable", name = "SM_Object")]
    sm_object: StaticMeshComponent,

    #[component(attach = "placeable", name = "HUDInteraction")]
    hud_interaction: WidgetComponent,

    #[component(attach = "scene", name = "SM_ArrowSpline")]
    sm_arrow_spline: SplineMeshComponent,

    #[component(attach = "scene", name = "SM_ArrowHead")]
    sm_arrow_head: StaticMeshComponent,

    #[component(name = "BP_ProductAssetRef")]
    bp_product_asset_ref: ProductAssetRef,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Snapping", default = 5.0)]
    snap_grid_size_position: f32,

    #[uproperty(
        EditAnywhere,
        BlueprintReadWrite,
        category = "Snapping",
        default = 20.0
    )]
    snap_grid_size_rotation: f32,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Snapping", default = 0.1)]
    snap_grid_size_scale: f32,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Visuals")]
    placeable_material: UObjectRef<MaterialInterface>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Visuals")]
    axis_texture: UObjectRef<Texture>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Visuals")]
    circle_plate_texture: UObjectRef<Texture>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Visuals")]
    square_plate_texture: UObjectRef<Texture>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Visuals")]
    move_icon: UObjectRef<Texture2D>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Visuals")]
    rotate_icon: UObjectRef<Texture2D>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Visuals")]
    scale_icon: UObjectRef<Texture2D>,

    #[uproperty(BlueprintReadWrite, name = "BP_Plane", category = "Placeable")]
    bp_plane: UObjectRef<Plane>,

    #[uproperty(BlueprintReadOnly, name = "DM_Plate")]
    dm_plate: UObjectRef<MaterialInstanceDynamic>,

    #[uproperty(BlueprintReadOnly, category = "Placeable", default = 1.0)]
    previous_placeable_scale: f32,

    #[uproperty(BlueprintReadOnly, category = "Placeable", default = 0.0)]
    previous_scale_mapping: f32,

    #[uproperty(BlueprintReadOnly, category = "Placeable", default = false)]
    visuals_are_set: bool,

    #[uproperty(BlueprintReadOnly, category = "Placeable", default = false)]
    is_interacting: bool,

    #[uproperty(BlueprintReadOnly, category = "Placeable", default = EGestureType::Translation)]
    interaction_type: EGestureType,

    #[uproperty(BlueprintReadOnly, category = "Placeable")]
    arrow_start_position: OwnedStruct<FVector>,

    rotation: Rotation,
}

#[uclass_impl]
impl Placeable {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let no_collision = FName::new("NoCollision").handle();

        let shadow = self.sm_shadow()?.checked()?;
        shadow.set_cast_shadow(false);
        shadow.set_collision_profile_name(no_collision, Some(true));
        shadow.set_translucency_sort_priority(1);

        self.sm_plate()?.checked()?.k2_set_relative_location(
            &FVector::from_dvec3(DVec3::new(0.0, 0.0, -1.0)),
            false,
            false,
        );

        let axis = self.sm_axis()?.checked()?;
        axis.k2_set_relative_location(
            &FVector::from_dvec3(DVec3::new(0.0, 0.0, 11.184432)),
            false,
            false,
        );
        axis.set_relative_scale3_d(&FVector::from_dvec3(DVec3::new(0.004, 0.004, 0.25)));

        let hud = self.hud_interaction()?.checked()?;
        hud.k2_set_relative_location(
            &FVector::from_dvec3(DVec3::new(0.0, 0.0, 22.0)),
            false,
            false,
        );
        hud.set_relative_scale3_d(&FVector::from_dvec3(DVec3::splat(0.5)));

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let Err(e) = self.begin_play() {
            ulog!(LOG_WARNING, "[AR] placeable begin play failed: {e}");
        }
    }

    #[ufunction(Override)]
    fn receive_tick(&mut self, _delta_seconds: f32) {
        if self.is_interacting() {
            self.update_interaction();
        }
    }

    #[ufunction(Override)]
    fn user_construction_script(&mut self) {
        self.assign_product_asset();
    }

    #[ufunction(BlueprintCallable)]
    pub fn set_placeable_position(
        &mut self,
        screen_position: UStructRef<FVector2D>,
        is_snapping: bool,
        show_hud: bool,
    ) {
        self.set_visuals(EGestureType::Translation, show_hud);

        if let Some(location) = self.trace_on_plane(screen_position.to_owned().to_dvec2()) {
            self.move_on_plane(FVector::from_dvec3(location).as_ref(), is_snapping);
        }
    }

    #[ufunction(BlueprintCallable)]
    pub fn move_on_plane(&mut self, location: UStructRef<FVector>, is_snapping: bool) {
        let snapped = model::snap(
            location.to_owned().to_dvec3(),
            f64::from(self.snap_grid_size_position()),
            is_snapping,
        );

        if let Ok(placeable) = self.placeable().and_then(|p| p.checked()) {
            placeable.k2_set_world_location(&FVector::from_dvec3(snapped), false, false);
        }

        self.update_arrow();
    }

    #[ufunction(BlueprintCallable)]
    pub fn set_placeable_rotation(
        &mut self,
        direction: UStructRef<FVector2D>,
        location: UStructRef<FVector2D>,
        is_snapping: bool,
        show_hud: bool,
    ) {
        self.set_visuals(EGestureType::Rotation, show_hud);

        let mut rotation = self.rotation();
        let yaw = rotation.step(
            direction.to_owned().to_dvec2(),
            location.to_owned().to_dvec2(),
            is_snapping,
            f64::from(self.snap_grid_size_rotation()),
        );
        self.set_rotation(rotation);

        if let (Some(yaw), Ok(placeable)) = (yaw, self.placeable().and_then(|p| p.checked())) {
            placeable.k2_add_relative_rotation(
                &FRotator::from_rotator(Rotator::new(0.0, yaw, 0.0)),
                false,
                false,
            );
        }
    }

    #[ufunction(BlueprintCallable)]
    pub fn set_placeable_scale(
        &mut self,
        new_scale_mapping: f32,
        is_snapping: bool,
        show_hud: bool,
    ) {
        self.set_visuals(EGestureType::Scale, show_hud);

        let scale = model::next_scale(
            f64::from(self.previous_placeable_scale()),
            f64::from(self.previous_scale_mapping()),
            f64::from(new_scale_mapping),
        );
        self.set_previous_placeable_scale(scale as f32);

        let snapped = model::snap(
            DVec3::splat(scale),
            f64::from(self.snap_grid_size_scale()),
            is_snapping,
        );

        if let Ok(placeable) = self.placeable().and_then(|p| p.checked()) {
            placeable.set_world_scale3_d(&FVector::from_dvec3(snapped));
        }

        self.set_previous_scale_mapping(new_scale_mapping);
    }

    #[ufunction(BlueprintCallable)]
    pub fn reset_visuals(&mut self) {
        self.set_is_interacting(false);
        self.set_visuals_are_set(false);

        if let Ok(hud) = self.hud_interaction().and_then(|h| h.checked()) {
            hud.set_visibility(false, Some(false));
        }

        for component in [
            self.sm_axis(),
            self.sm_plate(),
            self.sm_arrow_spline().map(|c| c.upcast_to()),
            self.sm_arrow_head(),
        ] {
            if let Ok(component) = component.and_then(|c| c.checked()) {
                component.set_visibility(false, Some(false));
            }
        }

        self.set_rotation(Rotation::default());

        if let Ok(plane) = Plane::from_obj(self.bp_plane()) {
            plane.hide();
        }
    }
}

impl Placeable {
    fn begin_play(&mut self) -> RustealResult<()> {
        self.assign_product_asset();
        self.reset_visuals();

        let me = self.as_ref();
        let plate = KismetMaterialLibrary::create_dynamic_material_instance(
            me.upcast_to(),
            self.placeable_material(),
            None,
            None,
        );
        self.sm_plate()?
            .checked()?
            .set_material(0, plate.upcast_to());
        self.set_dm_plate(plate);

        let axis = KismetMaterialLibrary::create_dynamic_material_instance(
            me.upcast_to(),
            self.placeable_material(),
            None,
            None,
        );
        axis.checked()?
            .set_texture_parameter_value(FName::new("Texture").handle(), self.axis_texture());
        self.sm_axis()?.checked()?.set_material(0, axis.upcast_to());
        Ok(())
    }

    fn assign_product_asset(&self) {
        let Ok(product) = self
            .bp_product_asset_ref()
            .and_then(ProductAssetRef::from_obj)
        else {
            return;
        };

        if let Ok(object) = self.sm_object().and_then(|o| o.checked()) {
            object.set_static_mesh(product.static_mesh());
        }

        let scale = FVector::from_dvec3(DVec3::new(
            f64::from(product.plate_scale()),
            f64::from(product.plate_scale()),
            1.0,
        ));

        for component in [self.sm_plate(), self.sm_shadow()] {
            if let Ok(component) = component.and_then(|c| c.checked()) {
                component.set_world_scale3_d(&scale);
            }
        }
    }

    fn trace_on_plane(&self, screen_position: DVec2) -> Option<DVec3> {
        let me = self.as_ref();
        let controller = GameplayStatics::get_player_controller(me.upcast_to(), 0);
        let (_, origin, direction) = GameplayStatics::deproject_screen_to_world(
            controller,
            &FVector2D::from_dvec2(screen_position),
        );
        let start = origin.to_dvec3();
        let end = start + direction.to_dvec3() * model::HIT_TEST_DISTANCE;
        let white = FLinearColor::from_linear_color(LinearColor::new(1.0, 1.0, 1.0, 1.0));

        let (hit, result) = KismetSystemLibrary::line_trace_single_for_objects(
            me.upcast_to(),
            &FVector::from_dvec3(start),
            &FVector::from_dvec3(end),
            &[
                EObjectTypeQuery::ObjectTypeQuery1,
                EObjectTypeQuery::ObjectTypeQuery2,
            ],
            false,
            &[],
            EDrawDebugTrace::None,
            true,
            &white,
            &white,
            None,
        );

        if !hit {
            return None;
        }

        let result = result.as_ref();
        let actor = result.get_component().checked().ok()?.get_owner();

        (actor == self.bp_plane().upcast_to()).then(|| result.get_location().to_dvec3())
    }

    fn set_visuals(&mut self, gesture: EGestureType, show_hud: bool) {
        if self.visuals_are_set() {
            return;
        }

        self.set_visuals_are_set(true);
        self.set_is_interacting(true);
        self.set_interaction_type(gesture);

        if show_hud {
            self.set_interaction_icon();
        }

        match gesture {
            EGestureType::Translation => {
                if let Ok(plane) = Plane::from_obj(self.bp_plane()) {
                    plane.show();
                    plane.switch_to_material_translate();
                    self.init_arrow();
                }
            }
            EGestureType::Rotation => {
                for component in [self.sm_axis(), self.sm_plate()] {
                    if let Ok(component) = component.and_then(|c| c.checked()) {
                        component.set_visibility(true, Some(true));
                    }
                }

                self.set_plate_texture(self.circle_plate_texture());
            }
            EGestureType::Scale => {
                if let Ok(plate) = self.sm_plate().and_then(|c| c.checked()) {
                    plate.set_visibility(true, Some(false));
                }

                self.set_plate_texture(self.square_plate_texture());
            }
        }
    }

    fn set_plate_texture(&self, texture: UObjectRef<Texture>) {
        if let Ok(plate) = self.dm_plate().checked() {
            plate.set_texture_parameter_value(FName::new("Texture").handle(), texture);
        }
    }

    fn hud(&self) -> Option<InteractionHUD> {
        let widget = self.hud_interaction().ok()?.checked().ok()?.get_widget();
        InteractionHUD::from_obj(widget).ok()
    }

    fn set_interaction_icon(&self) {
        if let Ok(hud) = self.hud_interaction().and_then(|h| h.checked()) {
            hud.set_visibility(true, Some(false));
        }

        let icon = match self.interaction_type() {
            EGestureType::Translation => self.move_icon(),
            EGestureType::Rotation => self.rotate_icon(),
            EGestureType::Scale => self.scale_icon(),
        };

        if let Some(hud) = self.hud() {
            hud.set_icon(icon);
        }
    }

    fn update_interaction(&self) {
        let Some(hud) = self.hud() else {
            return;
        };

        match self.interaction_type() {
            EGestureType::Translation => hud.show(&self.location_text(), "cm"),
            EGestureType::Rotation => hud.show(&self.rotation_text(), "deg"),
            EGestureType::Scale => hud.show(&self.scale_text(), "cm"),
        }
    }

    fn number(value: f64) -> String {
        format!("{}", value.round() as i64)
    }

    fn location_text(&self) -> String {
        let head = self
            .sm_arrow_head()
            .and_then(|c| c.checked())
            .map(|c| c.k2_get_component_location().to_dvec3());
        let spline = self
            .sm_arrow_spline()
            .and_then(|c| c.upcast_to::<SceneComponent>().checked())
            .map(|c| c.k2_get_component_location().to_dvec3());

        match (head, spline) {
            (Ok(head), Ok(spline)) => Self::number(head.distance(spline)),
            _ => String::new(),
        }
    }

    fn rotation_text(&self) -> String {
        self.placeable()
            .and_then(|p| p.checked())
            .map(|p| Self::number(p.get_relative_rotation().to_rotator().yaw + 180.0))
            .unwrap_or_default()
    }

    fn scale_text(&self) -> String {
        let (Ok(placeable), Ok(object)) = (
            self.placeable().and_then(|p| p.checked()),
            self.sm_object().and_then(|o| o.checked()),
        ) else {
            return String::new();
        };

        let (min, max) = object.get_local_bounds();
        let size = max.to_dvec3() - min.to_dvec3();
        Self::number(placeable.k2_get_component_scale().to_dvec3().x * size.x.max(size.y))
    }

    fn init_arrow(&mut self) {
        let (Ok(spline), Ok(head), Ok(placeable)) = (
            self.sm_arrow_spline().and_then(|c| c.checked()),
            self.sm_arrow_head().and_then(|c| c.checked()),
            self.placeable().and_then(|p| p.checked()),
        ) else {
            return;
        };

        spline.set_visibility(true, Some(true));
        head.set_visibility(true, Some(true));

        let start = placeable.k2_get_component_location();
        self.set_arrow_start_position(&start);
        spline.set_start_position(&FVector::from_dvec3(DVec3::ZERO), Some(true));
        spline.k2_set_world_location(&start, false, false);
        head.k2_set_world_location(&start, false, false);
    }

    fn update_arrow(&self) {
        let (Ok(spline), Ok(head), Ok(placeable)) = (
            self.sm_arrow_spline().and_then(|c| c.checked()),
            self.sm_arrow_head().and_then(|c| c.checked()),
            self.placeable().and_then(|p| p.checked()),
        ) else {
            return;
        };

        let offset = placeable.k2_get_component_location().to_dvec3()
            - self.arrow_start_position().to_dvec3();
        let tangent = FVector::from_dvec3(offset.normalize_or_zero());
        spline.set_start_tangent(&tangent, Some(true));
        spline.set_end_tangent(&tangent, Some(true));
        spline.set_end_position(&FVector::from_dvec3(offset), Some(true));

        let head_location = placeable.k2_get_component_location().to_dvec3()
            + spline.get_start_tangent().to_dvec3();
        head.k2_set_world_location(&FVector::from_dvec3(head_location), false, false);

        let look = KismetMathLibrary::find_look_at_rotation(
            &head.k2_get_component_location(),
            &spline.k2_get_component_location(),
        );
        head.k2_set_relative_rotation(&look, false, false);
    }
}
