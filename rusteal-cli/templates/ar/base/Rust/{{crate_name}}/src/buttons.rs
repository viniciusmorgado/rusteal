use bindings::engine::{
    KismetMaterialLibrary, MaterialInstanceDynamic, MaterialInstanceDynamicExt, MaterialInterface,
    Texture2D,
};
use bindings::prelude::*;
use bindings::slate_core::{
    ESlateBrushDrawType, FButtonStyle, FButtonStyleExt, FSlateBrush, FSlateBrushExt,
};
use bindings::umg::{
    Button, ButtonExt, ESlateVisibility, Image, ImageExt, TextBlock, TextBlockExt, UserWidget,
    WidgetBlueprintLibrary, WidgetExt,
};
use rusteal_runtime::runtime::{
    FName, LinearColor, OwnedStruct, RustealResult, UObjectRef, method,
};
use rusteal_runtime::{uclass, uclass_impl};

use crate::enums::{EIconType, EUIStyle};
use crate::styles::{StylizedUI, font_info, slate_color};

fn dynamic_material(
    owner: UObjectRef<UserWidget>,
    parent: UObjectRef<MaterialInterface>,
) -> UObjectRef<MaterialInstanceDynamic> {
    KismetMaterialLibrary::create_dynamic_material_instance(owner.upcast_to(), parent, None, None)
}

fn material_from_image(
    owner: UObjectRef<UserWidget>,
    parent: UObjectRef<MaterialInterface>,
    image: UObjectRef<Image>,
) -> RustealResult<UObjectRef<MaterialInstanceDynamic>> {
    let image = image.checked()?;
    let material = dynamic_material(owner, parent);
    let icon = WidgetBlueprintLibrary::get_brush_resource_as_texture2_d(&image.get_brush());

    material
        .checked()?
        .set_texture_parameter_value(FName::new("Icon").handle(), icon.upcast_to());
    image.set_brush_from_material(material.upcast_to());
    Ok(material)
}

#[uclass(parent = StylizedUI)]
pub struct CapsuleButton {
    #[uproperty(BindWidget, BlueprintReadOnly, name = "UI_Text")]
    ui_text: UObjectRef<TextBlock>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UI_Button")]
    ui_button: UObjectRef<Button>,

    #[uproperty(BindWidget, BlueprintReadOnly)]
    capsule_left: UObjectRef<Image>,

    #[uproperty(BindWidget, BlueprintReadOnly)]
    capsule_center: UObjectRef<Image>,

    #[uproperty(BindWidget, BlueprintReadOnly)]
    capsule_right: UObjectRef<Image>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Button")]
    button_master_material: UObjectRef<MaterialInterface>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Button")]
    normal_color: OwnedStruct<FLinearColor>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Button")]
    pressed_color: OwnedStruct<FLinearColor>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Button")]
    text: String,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Button", default = true)]
    is_pressable: bool,

    #[uproperty(BlueprintReadOnly, name = "DM_CapsuleLeft")]
    dm_capsule_left: UObjectRef<MaterialInstanceDynamic>,

    #[uproperty(BlueprintReadOnly, name = "DM_CapsuleCenter")]
    dm_capsule_center: UObjectRef<MaterialInstanceDynamic>,

    #[uproperty(BlueprintReadOnly, name = "DM_CapsuleRight")]
    dm_capsule_right: UObjectRef<MaterialInstanceDynamic>,
}

#[uclass_impl]
impl CapsuleButton {
    #[udelegate]
    pub fn on_pressed_event(&self) {}

    #[udelegate]
    pub fn on_release_event(&self) {}

    #[ufunction(Override)]
    fn on_initialized(&mut self) {
        if let Err(e) = self.init() {
            log::warn!("[AR] capsule button init failed: {e}");
        }
    }

    #[ufunction]
    fn button_pressed(&mut self) {
        if let Ok(text) = self.ui_text().checked() {
            text.set_color_and_opacity(&slate_color(self.pressed_color().to_linear_color()));
        }

        self.on_pressed_event();
    }

    #[ufunction]
    fn button_released(&mut self) {
        if let Ok(text) = self.ui_text().checked() {
            text.set_color_and_opacity(&slate_color(self.normal_color().to_linear_color()));
        }

        self.on_release_event();
    }

    #[ufunction(BlueprintCallable)]
    pub fn change_style(&mut self, style: EUIStyle) {
        let Ok(base) = StylizedUI::from_obj(self.as_ref()) else {
            return;
        };

        let data = base.get_style_data(style);

        if let Ok(text) = self.ui_text().checked() {
            text.set_font(&font_info(&data));
        }

        let color = FLinearColor::from_linear_color(data.bg_color);

        for (material, icon) in [
            (self.dm_capsule_left(), EIconType::CapsuleLeft),
            (self.dm_capsule_center(), EIconType::CapsuleCenter),
            (self.dm_capsule_right(), EIconType::CapsuleRight),
        ] {
            if let Ok(material) = material.checked() {
                material.set_vector_parameter_value(FName::new("Color").handle(), &color);
                material.set_texture_parameter_value(
                    FName::new("Icon").handle(),
                    data.icon(icon).upcast_to(),
                );
            }
        }
    }
}

impl CapsuleButton {
    fn init(&mut self) -> RustealResult<()> {
        let me = self.as_ref().upcast_to::<UserWidget>();
        self.ui_text().checked()?.set_text(&self.text());

        let master = self.button_master_material();
        self.set_dm_capsule_left(material_from_image(me, master, self.capsule_left())?);
        self.set_dm_capsule_center(material_from_image(me, master, self.capsule_center())?);
        self.set_dm_capsule_right(material_from_image(me, master, self.capsule_right())?);

        let button = self.ui_button().checked()?;

        if !self.is_pressable() {
            button.set_visibility(ESlateVisibility::Hidden);
        }

        button
            .on_pressed()
            .add(method(self, Self::button_pressed))?
            .detach();
        button
            .on_released()
            .add(method(self, Self::button_released))?
            .detach();
        Ok(())
    }
}

#[uclass(parent = StylizedUI)]
pub struct ToggleButton {
    #[uproperty(BindWidget, BlueprintReadOnly)]
    button: UObjectRef<Button>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Button")]
    icon_on: UObjectRef<Texture2D>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Button")]
    icon_off: UObjectRef<Texture2D>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, name = "IconTypeON", category = "Button", default = EIconType::Enable)]
    icon_type_on: EIconType,

    #[uproperty(EditAnywhere, BlueprintReadWrite, name = "IconTypeOFF", category = "Button", default = EIconType::Enable)]
    icon_type_off: EIconType,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Button")]
    color_on: OwnedStruct<FLinearColor>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Button")]
    color_off: OwnedStruct<FLinearColor>,

    #[uproperty(
        BlueprintReadWrite,
        name = "isButtonPressed",
        category = "Button",
        default = false
    )]
    is_button_pressed: bool,

    #[uproperty(
        EditAnywhere,
        BlueprintReadWrite,
        name = "ID",
        category = "Button",
        default = 0
    )]
    id: i32,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Button", default = 64)]
    button_width: i32,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Button", default = 64)]
    button_height: i32,

    #[uproperty(
        EditAnywhere,
        BlueprintReadWrite,
        name = "isToggleable",
        category = "Button",
        default = true
    )]
    is_toggleable: bool,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Button", default = false)]
    default_value: bool,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Button")]
    button_master_material: UObjectRef<MaterialInterface>,

    #[uproperty(BlueprintReadOnly, name = "DM_Button")]
    dm_button: UObjectRef<MaterialInstanceDynamic>,
}

#[uclass_impl]
impl ToggleButton {
    #[udelegate]
    pub fn on_pressed_event(&self, button_id: i32) {}

    #[udelegate]
    pub fn on_release_event(&self, is_pressed: bool, button_id: i32) {}

    #[ufunction(Override)]
    fn on_initialized(&mut self) {
        if let Err(e) = self.init_button() {
            log::warn!("[AR] toggle button init failed: {e}");
        }
    }

    #[ufunction]
    fn button_pressed(&mut self) {
        if !self.is_toggleable() {
            self.switch_icon_on();
        }

        self.on_pressed_event(self.id());
    }

    #[ufunction]
    fn button_released(&mut self) {
        self.toggle();
        self.on_release_event(self.is_button_pressed(), self.id());
    }

    #[ufunction(BlueprintCallable)]
    pub fn toggle(&mut self) {
        if !self.is_toggleable() {
            self.switch_icon_off();
            return;
        }

        let pressed = !self.is_button_pressed();
        self.set_is_button_pressed(pressed);

        if pressed {
            self.switch_icon_on();
        } else {
            self.switch_icon_off();
        }
    }

    #[ufunction(BlueprintCallable)]
    pub fn turn_off(&mut self) {
        self.set_is_button_pressed(false);
        self.switch_icon_off();
    }

    #[ufunction(BlueprintCallable, name = "TurnON")]
    pub fn turn_on(&mut self) {
        self.set_is_button_pressed(true);
        self.switch_icon_on();
    }

    #[ufunction(BlueprintCallable)]
    pub fn change_style(&mut self, style: EUIStyle) {
        let Ok(base) = StylizedUI::from_obj(self.as_ref()) else {
            return;
        };

        let data = base.get_style_data(style);
        self.set_color_on(&FLinearColor::from_linear_color(data.button_color_on));
        self.set_color_off(&FLinearColor::from_linear_color(data.button_color_off));
        self.set_icon_on(data.icon(self.icon_type_on()));
        self.set_icon_off(data.icon(self.icon_type_off()));

        if self.is_button_pressed() {
            self.switch_icon_on();
        } else {
            self.switch_icon_off();
        }
    }

    #[ufunction(BlueprintCallable)]
    pub fn switch_icon_on(&self) {
        self.show_icon(self.icon_on(), self.color_on().to_linear_color());
    }

    #[ufunction(BlueprintCallable)]
    pub fn switch_icon_off(&self) {
        self.show_icon(self.icon_off(), self.color_off().to_linear_color());
    }
}

impl ToggleButton {
    fn show_icon(&self, icon: UObjectRef<Texture2D>, color: LinearColor) {
        if let Ok(material) = self.dm_button().checked() {
            material.set_texture_parameter_value(FName::new("Icon").handle(), icon.upcast_to());
            material.set_vector_parameter_value(
                FName::new("Color").handle(),
                &FLinearColor::from_linear_color(color),
            );
        }
    }

    fn init_button(&mut self) -> RustealResult<()> {
        let me = self.as_ref().upcast_to::<UserWidget>();
        let material = dynamic_material(me, self.button_master_material());
        self.set_dm_button(material);
        self.set_is_button_pressed(self.default_value());

        let brush = WidgetBlueprintLibrary::make_brush_from_material(
            material.upcast_to(),
            Some(self.button_width()),
            Some(self.button_height()),
        );

        let disabled = OwnedStruct::<FSlateBrush>::new();
        disabled
            .as_ref()
            .set_draw_as(ESlateBrushDrawType::NoDrawType);

        let style = OwnedStruct::<FButtonStyle>::new();
        let style_ref = style.as_ref();
        style_ref.set_normal(&brush);
        style_ref.set_hovered(&brush);
        style_ref.set_pressed(&brush);
        style_ref.set_disabled(&disabled);

        let button = self.button().checked()?;
        button.set_style(&style);
        self.change_style(EUIStyle::Light);

        button
            .on_pressed()
            .add(method(self, Self::button_pressed))?
            .detach();
        button
            .on_released()
            .add(method(self, Self::button_released))?
            .detach();
        Ok(())
    }
}
