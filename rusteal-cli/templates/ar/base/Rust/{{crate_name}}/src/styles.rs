use bindings::core_ue::Object;
use bindings::engine::{
    DataTable, DataTableFunctionLibrary, MaterialInstanceDynamic, MaterialInstanceDynamicExt,
    Texture2D,
};
use bindings::prelude::*;
use bindings::slate_core::{
    ESlateColorStylingMode, FSlateColor, FSlateColorExt, FSlateFontInfo, FSlateFontInfoExt,
};
use bindings::umg::{Image, ImageExt, TextBlock, TextBlockExt, UserWidget};
use rusteal_runtime::runtime::{
    FName, LOG_WARNING, LinearColor, OwnedStruct, UObjectRef, UeArray, ulog,
};
use rusteal_runtime::{uclass, uclass_impl, ustruct};

use crate::buttons::{CapsuleButton, ToggleButton};
use crate::enums::{EFontTypeFace, EIconType, EUIStyle};

#[ustruct]
pub struct StructIcon {
    #[uproperty(EditAnywhere, name = "Type")]
    kind: EIconType,

    #[uproperty(EditAnywhere)]
    icon: UObjectRef<Texture2D>,
}

#[ustruct]
pub struct StructStyle {
    #[uproperty(EditAnywhere)]
    style: EUIStyle,

    #[uproperty(EditAnywhere)]
    button_color_on: OwnedStruct<FLinearColor>,

    #[uproperty(EditAnywhere)]
    button_color_off: OwnedStruct<FLinearColor>,

    #[uproperty(EditAnywhere, name = "BGColor")]
    bg_color: OwnedStruct<FLinearColor>,

    #[uproperty(EditAnywhere)]
    text_color: OwnedStruct<FLinearColor>,

    #[uproperty(EditAnywhere)]
    font: UObjectRef<Object>,

    #[uproperty(EditAnywhere)]
    font_type: EFontTypeFace,

    #[uproperty(EditAnywhere)]
    font_size: i32,

    #[uproperty(EditAnywhere)]
    icons: UeArray<OwnedStruct<StructIcon>>,
}

#[derive(Clone)]
pub struct StyleData {
    pub button_color_on: LinearColor,
    pub button_color_off: LinearColor,
    pub bg_color: LinearColor,
    pub text_color: LinearColor,
    pub font: UObjectRef<Object>,
    pub font_type: EFontTypeFace,
    pub font_size: i32,
    pub icons: Vec<(EIconType, UObjectRef<Texture2D>)>,
}

impl Default for StyleData {
    fn default() -> Self {
        let dark = LinearColor::new(0.1, 0.1, 0.1, 1.0);

        Self {
            button_color_on: dark,
            button_color_off: dark,
            bg_color: LinearColor::new(0.9, 0.9, 0.9, 1.0),
            text_color: dark,
            font: UObjectRef::null(),
            font_type: EFontTypeFace::Regular,
            font_size: 24,
            icons: Vec::new(),
        }
    }
}

impl StyleData {
    pub fn icon(&self, kind: EIconType) -> UObjectRef<Texture2D> {
        self.icons
            .iter()
            .find(|(icon_kind, _)| *icon_kind == kind)
            .map(|(_, icon)| *icon)
            .unwrap_or_default()
    }
}

#[uclass(parent = UserWidget)]
pub struct StylizedUI {
    #[uproperty(
        EditAnywhere,
        BlueprintReadWrite,
        name = "DT_Style",
        category = "Style"
    )]
    dt_style: UObjectRef<DataTable>,
}

#[uclass_impl]
impl StylizedUI {
    #[udelegate]
    pub fn switch_style(&self, style: EUIStyle) {}

    #[ufunction(BlueprintCallable)]
    pub fn call_switch_style(&self, new_style: EUIStyle) {
        self.switch_style(new_style);
    }
}

impl StylizedUI {
    pub fn get_style_data(&self, current_style: EUIStyle) -> StyleData {
        let table = self.dt_style();

        for row_name in DataTableFunctionLibrary::get_data_table_row_names(table) {
            let Some(row) = find_data_table_row::<StructStyle>(table, FName(row_name)) else {
                continue;
            };

            if row.style() != current_style {
                continue;
            }

            let icons = row
                .icons()
                .to_vec()
                .unwrap_or_default()
                .iter()
                .map(|icon| (icon.kind(), icon.icon()))
                .collect();

            return StyleData {
                button_color_on: row.button_color_on().to_linear_color(),
                button_color_off: row.button_color_off().to_linear_color(),
                bg_color: row.bg_color().to_linear_color(),
                text_color: row.text_color().to_linear_color(),
                font: row.font(),
                font_type: row.font_type(),
                font_size: row.font_size(),
                icons,
            };
        }

        ulog!(LOG_WARNING, "Style not found...");
        StyleData::default()
    }
}

pub fn slate_color(color: LinearColor) -> OwnedStruct<FSlateColor> {
    let slate = OwnedStruct::<FSlateColor>::new();
    let slate_ref = slate.as_ref();
    slate_ref.set_specified_color(&FLinearColor::from_linear_color(color));
    slate_ref.set_color_use_rule(ESlateColorStylingMode::UseColor_Specified);
    slate
}

pub fn font_info(style: &StyleData) -> OwnedStruct<FSlateFontInfo> {
    let font = OwnedStruct::<FSlateFontInfo>::new();
    let font_ref = font.as_ref();
    font_ref.set_font_object(style.font);
    font_ref.set_typeface_font_name(FName::new(style.font_type.typeface()).handle());
    font_ref.set_size(style.font_size as f32);
    font
}

pub fn set_text_style_array(
    texts: &[UObjectRef<TextBlock>],
    color: LinearColor,
    style: &StyleData,
) {
    let color = slate_color(color);
    let font = font_info(style);

    for text in texts {
        if let Ok(text) = text.checked() {
            text.set_color_and_opacity(&color);
            text.set_font(&font);
        }
    }
}

pub fn set_image_color_array(images: &[UObjectRef<Image>], color: LinearColor) {
    let color = FLinearColor::from_linear_color(color);

    for image in images {
        if let Ok(image) = image.checked() {
            image.set_color_and_opacity(&color);
        }
    }
}

pub fn set_material_color_array(
    materials: &[UObjectRef<MaterialInstanceDynamic>],
    color: LinearColor,
) {
    let color = FLinearColor::from_linear_color(color);

    for material in materials {
        if let Ok(material) = material.checked() {
            material.set_vector_parameter_value(FName::new("Color").handle(), &color);
        }
    }
}

pub fn set_toggle_button_style_array(buttons: &[UObjectRef<ToggleButton>], style: EUIStyle) {
    for button in buttons {
        if let Ok(mut button) = ToggleButton::from_obj(*button) {
            button.change_style(style);
        }
    }
}

pub fn set_capsule_style_array(capsules: &[UObjectRef<CapsuleButton>], style: EUIStyle) {
    for capsule in capsules {
        if let Ok(mut capsule) = CapsuleButton::from_obj(*capsule) {
            capsule.change_style(style);
        }
    }
}
