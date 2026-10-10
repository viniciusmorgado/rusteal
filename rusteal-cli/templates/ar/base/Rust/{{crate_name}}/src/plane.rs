use bindings::engine::{
    Actor, GameplayStatics, KismetMaterialLibrary, MaterialInstanceDynamic,
    MaterialInstanceDynamicExt, MaterialInterface, PrimitiveComponentExt, SceneComponent,
    SceneComponentExt, StaticMeshComponent, Texture,
};
use bindings::prelude::*;
use glam::DVec3;
use rusteal_runtime::runtime::{FName, LinearColor, UObjectRef};
use rusteal_runtime::{uclass, uclass_impl};

#[uclass(parent = Actor)]
pub struct Plane {
    #[component(root)]
    root: SceneComponent,

    #[component(attach = "root")]
    plane: StaticMeshComponent,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Plane")]
    scan_material01: UObjectRef<MaterialInterface>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Plane")]
    scan_material02: UObjectRef<MaterialInterface>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Plane")]
    fallback_material: UObjectRef<MaterialInterface>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Plane")]
    plain_material: UObjectRef<MaterialInterface>,

    #[uproperty(BlueprintReadOnly, name = "DM_Scan")]
    dm_scan: UObjectRef<MaterialInstanceDynamic>,

    #[uproperty(BlueprintReadOnly, name = "DM_Plain")]
    dm_plain: UObjectRef<MaterialInstanceDynamic>,
}

#[uclass_impl]
impl Plane {
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        let material = KismetMaterialLibrary::create_dynamic_material_instance(
            self.as_ref().upcast_to(),
            self.plain_material(),
            None,
            None,
        );
        self.set_dm_plain(material);
    }

    #[ufunction(BlueprintCallable)]
    pub fn show(&self) {
        self.set_plane_visible(true);
    }

    #[ufunction(BlueprintCallable)]
    pub fn hide(&self) {
        self.set_plane_visible(false);
    }

    #[ufunction(BlueprintCallable)]
    pub fn switch_to_material_translate(&self) {
        if let Ok(plane) = self.plane().and_then(|p| p.checked()) {
            plane.set_material(0, self.dm_plain().upcast_to());
        }
    }

    #[ufunction(BlueprintPure)]
    pub fn get_platform_scan_material(&self) -> UObjectRef<MaterialInterface> {
        match GameplayStatics::get_platform_name().as_str() {
            "Android" => self.scan_material01(),
            "IOS" => self.scan_material02(),
            _ => UObjectRef::null(),
        }
    }
}

impl Plane {
    fn set_plane_visible(&self, visible: bool) {
        if let Ok(plane) = self.plane().and_then(|p| p.checked()) {
            plane.set_visibility(visible, Some(false));
        }
    }

    pub fn is_shown(&self) -> bool {
        self.plane()
            .and_then(|p| p.checked())
            .is_ok_and(|p| p.is_visible())
    }

    pub fn initialize_plane(&mut self, depth_map: UObjectRef<Texture>, color: LinearColor) {
        let me = self.as_ref();

        let material = if depth_map.is_valid() {
            let material = KismetMaterialLibrary::create_dynamic_material_instance(
                me.upcast_to(),
                self.get_platform_scan_material(),
                None,
                None,
            );

            if let Ok(material) = material.checked() {
                material
                    .set_texture_parameter_value(FName::new("Camera Depth").handle(), depth_map);
            }

            material
        } else {
            KismetMaterialLibrary::create_dynamic_material_instance(
                me.upcast_to(),
                self.fallback_material(),
                None,
                None,
            )
        };

        self.set_dm_scan(material);

        if let Ok(plane) = self.plane().and_then(|p| p.checked()) {
            plane.set_material(0, material.upcast_to());
        }

        if let Ok(material) = material.checked() {
            material.set_vector_parameter_value(
                FName::new("Color").handle(),
                &FLinearColor::from_linear_color(color),
            );
        }
    }

    pub fn set_scale(&self, scale: DVec3) {
        if let Ok(plane) = self.plane().and_then(|p| p.checked()) {
            plane.set_relative_scale3_d(&FVector::from_dvec3(DVec3::new(scale.x, scale.y, 1.0)));
        }
    }
}
