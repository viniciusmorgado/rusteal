use bindings::augmented_reality::{
    ARBlueprintLibrary, ARSessionConfig, EARSessionStatus, FARSessionStatusExt,
};
use std::time::Duration;

use bindings::engine::{
    GameplayStatics, KismetMaterialLibrary, KismetSystemLibrary, MaterialInstanceDynamic,
    MaterialInstanceDynamicExt, MaterialInterface, Texture2D,
};
use bindings::prelude::*;
use bindings::slate_core::FGeometry;
use bindings::umg::{
    Border, BorderExt, ESlateVisibility, EUMGSequencePlayMode, Image, ImageExt, Overlay, TextBlock,
    TextBlockExt, UserWidget, UserWidgetExt, Widget, WidgetAnimation, WidgetAnimationExt,
    WidgetExt,
};
use rusteal_runtime::runtime::timer::set_timer;
use rusteal_runtime::runtime::{FName, Inherits, RustealResult, UObjectRef, UStructRef, method};
use rusteal_runtime::{uclass, uclass_impl};

use crate::buttons::{CapsuleButton, ToggleButton};
use crate::enums::EUIStyle;
use crate::model;
use crate::pawn::ARPawn;
use crate::plane::Plane;
use crate::styles::{
    StylizedUI, set_capsule_style_array, set_image_color_array, set_material_color_array,
    set_text_style_array, set_toggle_button_style_array,
};

fn show<T: Inherits<Widget>>(widget: UObjectRef<T>, visible: bool) {
    if let Ok(widget) = widget.upcast_to::<Widget>().checked() {
        widget.set_visibility(if visible {
            ESlateVisibility::Visible
        } else {
            ESlateVisibility::Hidden
        });
    }
}

fn play<T: Inherits<UserWidget>>(
    widget: UObjectRef<T>,
    animation: UObjectRef<WidgetAnimation>,
    finished: Option<Box<dyn FnMut() + Send>>,
) {
    let Ok(user_widget) = widget.upcast_to::<UserWidget>().checked() else {
        return;
    };

    user_widget.play_animation(
        animation,
        Some(0.0),
        Some(1),
        Some(EUMGSequencePlayMode::Forward),
        Some(1.0),
        Some(false),
    );

    if let (Some(finished), Ok(animation)) = (finished, animation.checked()) {
        let delay = Duration::from_secs_f32(animation.get_end_time().max(0.01));

        if let Err(e) = set_timer(&widget, delay, false, finished) {
            log::warn!("[AR] the animation end timer was not set: {e}");
        }
    }
}

fn bind_released(
    button: UObjectRef<ToggleButton>,
    callback: impl FnMut(bool, i32) + Send + 'static,
) -> RustealResult<()> {
    ToggleButton::from_obj(button)?
        .on_release_event_add(callback)?
        .detach();
    Ok(())
}

fn bind_style(
    widget: UObjectRef<UserWidget>,
    callback: impl FnMut(EUIStyle) + Send + 'static,
) -> RustealResult<()> {
    StylizedUI::from_obj(widget)?
        .switch_style_add(callback)?
        .detach();
    Ok(())
}

#[uclass(parent = StylizedUI)]
pub struct StartMenu {
    #[uproperty(BindWidget, BlueprintReadOnly, name = "UI_Scanning")]
    ui_scanning: UObjectRef<Overlay>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UI_StartSession")]
    ui_start_session: UObjectRef<Overlay>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UI_SelectPlane")]
    ui_select_plane: UObjectRef<Overlay>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UI_PlaceObject")]
    ui_place_object: UObjectRef<Overlay>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UI_LoadingCircle")]
    ui_loading_circle: UObjectRef<Image>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UI_CapsuleButton")]
    ui_capsule_button: UObjectRef<CapsuleButton>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UI_CapsuleButton_1")]
    ui_capsule_button_1: UObjectRef<CapsuleButton>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UI_CapsuleButton_2")]
    ui_capsule_button_2: UObjectRef<CapsuleButton>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UI_CapsuleButton_45")]
    ui_capsule_button_45: UObjectRef<CapsuleButton>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "TextBlock")]
    text_block: UObjectRef<TextBlock>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "TextBlock_0")]
    text_block_0: UObjectRef<TextBlock>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "TextBlock_54")]
    text_block_54: UObjectRef<TextBlock>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "TextBlock_67")]
    text_block_67: UObjectRef<TextBlock>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Scanning")]
    scanning_material: UObjectRef<MaterialInterface>,

    #[uproperty(BlueprintReadOnly, name = "DM_Scanning")]
    dm_scanning: UObjectRef<MaterialInstanceDynamic>,
}

#[uclass_impl]
impl StartMenu {
    #[udelegate]
    pub fn start_session(&self) {}

    #[ufunction(Override)]
    fn on_initialized(&mut self) {
        if let Err(e) = self.init() {
            log::warn!("[AR] start menu init failed: {e}");
        }
    }

    #[ufunction]
    fn start_ar(&mut self) {
        self.start_session();
    }

    #[ufunction(name = "SwitchUI")]
    fn switch_ui(&mut self, style: EUIStyle) {
        let Ok(base) = StylizedUI::from_obj(self.as_ref()) else {
            return;
        };

        let data = base.get_style_data(style);

        set_capsule_style_array(
            &[
                self.ui_capsule_button(),
                self.ui_capsule_button_1(),
                self.ui_capsule_button_2(),
                self.ui_capsule_button_45(),
            ],
            style,
        );

        set_text_style_array(
            &[
                self.text_block(),
                self.text_block_0(),
                self.text_block_67(),
                self.text_block_54(),
            ],
            data.button_color_off,
            &data,
        );
    }

    #[ufunction(BlueprintCallable)]
    pub fn to_scanning_state(&self) {
        show(self.ui_start_session(), false);
        show(self.ui_scanning(), true);
    }

    #[ufunction(BlueprintCallable)]
    pub fn to_select_plane_state(&self) {
        for overlay in [
            self.ui_scanning(),
            self.ui_place_object(),
            self.ui_start_session(),
        ] {
            show(overlay, false);
        }

        show(self.ui_select_plane(), true);
    }

    #[ufunction(BlueprintCallable)]
    pub fn to_place_object_state(&self) {
        for overlay in [
            self.ui_select_plane(),
            self.ui_start_session(),
            self.ui_scanning(),
        ] {
            show(overlay, false);
        }

        show(self.ui_place_object(), true);
    }
}

impl StartMenu {
    fn init(&mut self) -> RustealResult<()> {
        let me = self.as_ref().upcast_to::<UserWidget>();
        let circle = self.ui_loading_circle().checked()?;
        let material = KismetMaterialLibrary::create_dynamic_material_instance(
            me.upcast_to(),
            self.scanning_material(),
            None,
            None,
        );
        let icon = bindings::umg::WidgetBlueprintLibrary::get_brush_resource_as_texture2_d(
            &circle.get_brush(),
        );

        material
            .checked()?
            .set_texture_parameter_value(FName::new("Icon").handle(), icon.upcast_to());
        circle.set_brush_from_material(material.upcast_to());
        self.set_dm_scanning(material);

        CapsuleButton::from_obj(self.ui_capsule_button())?
            .on_release_event_add(method(self, Self::start_ar))?
            .detach();
        bind_style(me, method(self, Self::switch_ui))
    }
}

#[uclass(parent = StylizedUI)]
pub struct BottomMenu {
    #[uproperty(BindWidget, BlueprintReadOnly)]
    circular_style: UObjectRef<Overlay>,

    #[uproperty(BindWidget, BlueprintReadOnly)]
    default_style: UObjectRef<Overlay>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UI_Tutorial")]
    ui_tutorial: UObjectRef<Overlay>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UICircular_tutorial")]
    ui_circular_tutorial: UObjectRef<Overlay>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UI_Options")]
    ui_options: UObjectRef<ToggleButton>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UI_Reset")]
    ui_reset: UObjectRef<ToggleButton>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UI_Snapshot")]
    ui_snapshot: UObjectRef<ToggleButton>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UI_Info")]
    ui_info: UObjectRef<ToggleButton>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UICircular_Info")]
    ui_circular_info: UObjectRef<ToggleButton>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UICircular_Options")]
    ui_circular_options: UObjectRef<ToggleButton>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UICircular_Reset")]
    ui_circular_reset: UObjectRef<ToggleButton>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UICircular_Snapshot")]
    ui_circular_snapshot: UObjectRef<ToggleButton>,

    #[uproperty(BindWidget, BlueprintReadOnly)]
    capsule_left: UObjectRef<Image>,

    #[uproperty(BindWidget, BlueprintReadOnly)]
    capsule_center: UObjectRef<Image>,

    #[uproperty(BindWidget, BlueprintReadOnly)]
    capsule_right: UObjectRef<Image>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "Image_CircularMenu")]
    image_circular_menu: UObjectRef<Image>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "InfoBG")]
    info_bg: UObjectRef<Image>,

    #[uproperty(BindWidgetAnim, BlueprintReadOnly)]
    show_default_tutorial: UObjectRef<WidgetAnimation>,

    #[uproperty(BindWidgetAnim, BlueprintReadOnly)]
    show_circular_tutorial: UObjectRef<WidgetAnimation>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Menu")]
    button_master_material: UObjectRef<MaterialInterface>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Menu")]
    circular_menu_material: UObjectRef<MaterialInterface>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Menu")]
    circular_menu_texture: UObjectRef<Texture2D>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Menu")]
    circular_menu_top_texture: UObjectRef<Texture2D>,

    #[uproperty(BlueprintReadOnly, name = "DM_CapsuleLeft")]
    dm_capsule_left: UObjectRef<MaterialInstanceDynamic>,

    #[uproperty(BlueprintReadOnly, name = "DM_CapsuleCenter")]
    dm_capsule_center: UObjectRef<MaterialInstanceDynamic>,

    #[uproperty(BlueprintReadOnly, name = "DM_CapsuleRight")]
    dm_capsule_right: UObjectRef<MaterialInstanceDynamic>,

    #[uproperty(BlueprintReadOnly, name = "DM_Info")]
    dm_info: UObjectRef<MaterialInstanceDynamic>,

    #[uproperty(BlueprintReadOnly, name = "DM_CircularMenu")]
    dm_circular_menu: UObjectRef<MaterialInstanceDynamic>,
}

#[uclass_impl]
impl BottomMenu {
    #[udelegate]
    pub fn reset_pressed(&self) {}

    #[udelegate]
    pub fn screenshot_pressed(&self) {}

    #[udelegate]
    pub fn options_pressed(&self) {}

    #[udelegate]
    pub fn show_info_pressed(&self) {}

    #[ufunction(Override)]
    fn on_initialized(&mut self) {
        if let Err(e) = self.init() {
            log::warn!("[AR] bottom menu init failed: {e}");
        }
    }

    #[ufunction]
    fn show_options(&mut self, _is_pressed: bool, _button_id: i32) {
        self.options_pressed();
        self.update_circular_menu(-1);
    }

    #[ufunction]
    fn reset(&mut self, _is_pressed: bool, _button_id: i32) {
        self.reset_pressed();
        self.update_circular_menu(-1);
    }

    #[ufunction]
    fn show_info(&mut self, _is_pressed: bool, _button_id: i32) {
        self.show_info_pressed();
        self.update_circular_menu(-1);
    }

    #[ufunction]
    fn screen_grab(&mut self, _is_pressed: bool, _button_id: i32) {
        self.screenshot_pressed();
        self.update_circular_menu(-1);
    }

    #[ufunction]
    fn circular_menu_pressed(&mut self, button_id: i32) {
        self.update_circular_menu(button_id);
    }

    #[ufunction]
    fn anim_done(&mut self) {
        show(self.ui_tutorial(), false);
    }

    #[ufunction]
    fn anim_circular_tuto_done(&mut self) {
        show(self.ui_circular_tutorial(), false);
    }

    #[ufunction]
    fn style_switch(&mut self, style: EUIStyle) {
        let Ok(base) = StylizedUI::from_obj(self.as_ref()) else {
            return;
        };

        let data = base.get_style_data(style);

        set_toggle_button_style_array(
            &[
                self.ui_options(),
                self.ui_reset(),
                self.ui_snapshot(),
                self.ui_info(),
                self.ui_circular_info(),
                self.ui_circular_options(),
                self.ui_circular_reset(),
                self.ui_circular_snapshot(),
            ],
            style,
        );

        set_material_color_array(
            &[
                self.dm_capsule_left(),
                self.dm_capsule_center(),
                self.dm_capsule_right(),
                self.dm_info(),
            ],
            data.bg_color,
        );

        self.update_circular_menu(-1);

        let circular = style == EUIStyle::Game;
        show(self.circular_style(), circular);
        show(self.default_style(), !circular);
    }

    #[ufunction(BlueprintCallable)]
    pub fn play_tutorial(&mut self, style: EUIStyle) {
        let me = self.as_ref();

        if style == EUIStyle::Game {
            show(self.ui_circular_tutorial(), true);
            play(
                me,
                self.show_circular_tutorial(),
                Some(method(self, Self::anim_circular_tuto_done)),
            );
        } else {
            show(self.ui_tutorial(), true);
            play(
                me,
                self.show_default_tutorial(),
                Some(method(self, Self::anim_done)),
            );
        }
    }

    #[ufunction(BlueprintCallable)]
    pub fn update_circular_menu(&mut self, button_id: i32) {
        let Ok(material) = self.dm_circular_menu().checked() else {
            return;
        };

        material.set_scalar_parameter_value(
            FName::new("RotationAmount").handle(),
            model::circular_menu_rotation(button_id) as f32,
        );

        let texture = if button_id >= 0 {
            self.circular_menu_top_texture()
        } else {
            self.circular_menu_texture()
        };
        material
            .set_texture_parameter_value(FName::new("TextureMenu").handle(), texture.upcast_to());
    }

    #[ufunction(BlueprintCallable)]
    pub fn show_menu(&self) {
        show(self.as_ref(), true);
    }

    #[ufunction(BlueprintCallable)]
    pub fn hide_menu(&self) {
        show(self.as_ref(), false);
    }
}

impl BottomMenu {
    fn init(&mut self) -> RustealResult<()> {
        let me = self.as_ref().upcast_to::<UserWidget>();
        let master = self.button_master_material();

        self.set_dm_capsule_center(image_material(me, master, self.capsule_center())?);
        self.set_dm_capsule_left(image_material(me, master, self.capsule_left())?);
        self.set_dm_capsule_right(image_material(me, master, self.capsule_right())?);
        self.set_dm_info(image_material(me, master, self.info_bg())?);

        let circular = KismetMaterialLibrary::create_dynamic_material_instance(
            me.upcast_to(),
            self.circular_menu_material(),
            None,
            None,
        );
        self.image_circular_menu()
            .checked()?
            .set_brush_from_material(circular.upcast_to());
        self.set_dm_circular_menu(circular);

        for (button, callback) in [
            (self.ui_snapshot(), method(self, Self::screen_grab)),
            (self.ui_circular_snapshot(), method(self, Self::screen_grab)),
            (self.ui_reset(), method(self, Self::reset)),
            (self.ui_circular_reset(), method(self, Self::reset)),
            (self.ui_options(), method(self, Self::show_options)),
            (self.ui_circular_options(), method(self, Self::show_options)),
            (self.ui_info(), method(self, Self::show_info)),
            (self.ui_circular_info(), method(self, Self::show_info)),
        ] {
            bind_released(button, callback)?;
        }

        for button in [
            self.ui_circular_snapshot(),
            self.ui_circular_reset(),
            self.ui_circular_info(),
            self.ui_circular_options(),
        ] {
            ToggleButton::from_obj(button)?
                .on_pressed_event_add(method(self, Self::circular_menu_pressed))?
                .detach();
        }

        bind_style(me, method(self, Self::style_switch))
    }
}

fn image_material(
    owner: UObjectRef<UserWidget>,
    parent: UObjectRef<MaterialInterface>,
    image: UObjectRef<Image>,
) -> RustealResult<UObjectRef<MaterialInstanceDynamic>> {
    let image = image.checked()?;
    let material = KismetMaterialLibrary::create_dynamic_material_instance(
        owner.upcast_to(),
        parent,
        None,
        None,
    );
    let icon =
        bindings::umg::WidgetBlueprintLibrary::get_brush_resource_as_texture2_d(&image.get_brush());

    material
        .checked()?
        .set_texture_parameter_value(FName::new("Icon").handle(), icon.upcast_to());
    image.set_brush_from_material(material.upcast_to());
    Ok(material)
}

#[uclass(parent = StylizedUI)]
pub struct InfoMenu {
    #[uproperty(BindWidget, BlueprintReadOnly, name = "Border_170")]
    border_170: UObjectRef<Border>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UI_ButtonCloseInfo")]
    ui_button_close_info: UObjectRef<ToggleButton>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "Image_54")]
    image_54: UObjectRef<Image>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "Image_136")]
    image_136: UObjectRef<Image>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "Image_205")]
    image_205: UObjectRef<Image>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "Image_295")]
    image_295: UObjectRef<Image>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "Image_353")]
    image_353: UObjectRef<Image>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "Image_458")]
    image_458: UObjectRef<Image>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "TextBlock_84")]
    text_block_84: UObjectRef<TextBlock>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "TextBlock_154")]
    text_block_154: UObjectRef<TextBlock>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "TextBlock_241")]
    text_block_241: UObjectRef<TextBlock>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "TextBlock_337")]
    text_block_337: UObjectRef<TextBlock>,
}

#[uclass_impl]
impl InfoMenu {
    #[udelegate]
    pub fn close_info_menu(&self) {}

    #[ufunction(Override)]
    fn on_initialized(&mut self) {
        let me = self.as_ref().upcast_to::<UserWidget>();
        let bound = bind_released(self.ui_button_close_info(), method(self, Self::close))
            .and_then(|_| bind_style(me, method(self, Self::change_style)));

        if let Err(e) = bound {
            log::warn!("[AR] info menu init failed: {e}");
        }
    }

    #[ufunction]
    fn close(&mut self, _is_pressed: bool, _button_id: i32) {
        self.close_info_menu();
    }

    #[ufunction]
    fn change_style(&mut self, style: EUIStyle) {
        let Ok(base) = StylizedUI::from_obj(self.as_ref()) else {
            return;
        };

        let data = base.get_style_data(style);
        set_toggle_button_style_array(&[self.ui_button_close_info()], style);

        set_image_color_array(
            &[
                self.image_54(),
                self.image_136(),
                self.image_205(),
                self.image_295(),
                self.image_353(),
                self.image_458(),
            ],
            data.text_color,
        );

        set_text_style_array(
            &[
                self.text_block_84(),
                self.text_block_154(),
                self.text_block_241(),
                self.text_block_337(),
            ],
            data.text_color,
            &data,
        );

        if let Ok(border) = self.border_170().checked() {
            border.set_brush_color(&FLinearColor::from_linear_color(data.bg_color));
        }
    }
}

#[uclass(parent = StylizedUI)]
pub struct OptionMenu {
    #[uproperty(BindWidget, BlueprintReadOnly)]
    background: UObjectRef<Border>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "TextBlock_36")]
    text_block_36: UObjectRef<TextBlock>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "TextBlock_94")]
    text_block_94: UObjectRef<TextBlock>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "TextBlock_113")]
    text_block_113: UObjectRef<TextBlock>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "TextBlock_143")]
    text_block_143: UObjectRef<TextBlock>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UI_StyleText")]
    ui_style_text: UObjectRef<TextBlock>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UI_CloseButton")]
    ui_close_button: UObjectRef<ToggleButton>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UI_StyleLeftArrow")]
    ui_style_left_arrow: UObjectRef<ToggleButton>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UI_StyleRightArrow")]
    ui_style_right_arrow: UObjectRef<ToggleButton>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UI_ToggleHUD")]
    ui_toggle_hud: UObjectRef<ToggleButton>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UI_ToggleSnapping")]
    ui_toggle_snapping: UObjectRef<ToggleButton>,

    #[uproperty(BlueprintReadWrite, name = "UIStyle", category = "Style", default = EUIStyle::Light)]
    ui_style: EUIStyle,

    #[uproperty(
        BlueprintReadWrite,
        name = "UIStyleIndex",
        category = "Style",
        default = 0
    )]
    ui_style_index: i32,
}

#[uclass_impl]
impl OptionMenu {
    #[udelegate]
    pub fn close_options(&self) {}

    #[udelegate]
    pub fn toggle_hud_event(&self, hud: bool) {}

    #[udelegate]
    pub fn toggle_snap_event(&self, snap: bool) {}

    #[udelegate(name = "SwitchUIEvent")]
    pub fn switch_ui_event(&self, style: EUIStyle) {}

    #[ufunction(Override)]
    fn on_initialized(&mut self) {
        if let Err(e) = self.init() {
            log::warn!("[AR] option menu init failed: {e}");
        }
    }

    #[ufunction]
    fn switch(&mut self, style: EUIStyle) {
        let Ok(base) = StylizedUI::from_obj(self.as_ref()) else {
            return;
        };

        let data = base.get_style_data(style);

        set_text_style_array(
            &[
                self.text_block_36(),
                self.text_block_94(),
                self.text_block_143(),
                self.text_block_113(),
                self.ui_style_text(),
            ],
            data.text_color,
            &data,
        );

        set_toggle_button_style_array(
            &[
                self.ui_close_button(),
                self.ui_style_left_arrow(),
                self.ui_style_right_arrow(),
                self.ui_toggle_hud(),
                self.ui_toggle_snapping(),
            ],
            style,
        );

        if let Ok(background) = self.background().checked() {
            background.set_brush_color(&FLinearColor::from_linear_color(data.bg_color));
        }
    }

    #[ufunction]
    fn close(&mut self, _is_pressed: bool, _button_id: i32) {
        self.close_options();
    }

    #[ufunction(name = "HUD")]
    fn hud(&mut self, is_pressed: bool, _button_id: i32) {
        self.toggle_hud_event(is_pressed);
    }

    #[ufunction]
    fn snap(&mut self, is_pressed: bool, _button_id: i32) {
        self.toggle_snap_event(is_pressed);
    }

    #[ufunction]
    fn style_backward(&mut self, _is_pressed: bool, _button_id: i32) {
        self.set_ui_style_index((self.ui_style_index() + EUIStyle::COUNT - 1) % EUIStyle::COUNT);
        self.update_style_name();
        self.switch_ui_event(self.ui_style());
    }

    #[ufunction]
    fn style_forward(&mut self, _is_pressed: bool, _button_id: i32) {
        self.set_ui_style_index((self.ui_style_index() + 1) % EUIStyle::COUNT);
        self.update_style_name();
        self.switch_ui_event(self.ui_style());
    }

    #[ufunction(BlueprintCallable)]
    pub fn set_default_style(&mut self, new_style: EUIStyle) {
        self.set_ui_style_index(new_style.index());
        self.update_style_name();
    }

    #[ufunction(BlueprintCallable)]
    pub fn update_style_name(&mut self) {
        let style = EUIStyle::from_index(self.ui_style_index());
        self.set_ui_style(style);

        if let Ok(text) = self.ui_style_text().checked() {
            text.set_text(style.name());
        }
    }
}

impl OptionMenu {
    fn init(&mut self) -> RustealResult<()> {
        let me = self.as_ref().upcast_to::<UserWidget>();

        for (button, callback) in [
            (self.ui_close_button(), method(self, Self::close)),
            (self.ui_toggle_hud(), method(self, Self::hud)),
            (self.ui_toggle_snapping(), method(self, Self::snap)),
            (
                self.ui_style_left_arrow(),
                method(self, Self::style_backward),
            ),
            (
                self.ui_style_right_arrow(),
                method(self, Self::style_forward),
            ),
        ] {
            bind_released(button, callback)?;
        }

        bind_style(me, method(self, Self::switch))
    }
}

#[uclass(parent = UserWidget)]
pub struct MainMenu {
    #[uproperty(BindWidget, BlueprintReadOnly, name = "FullscreenBG")]
    fullscreen_bg: UObjectRef<Border>,

    #[uproperty(BindWidget, BlueprintReadOnly)]
    screenshot_overlay: UObjectRef<Overlay>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "BP_BottomMenu")]
    bp_bottom_menu: UObjectRef<BottomMenu>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "BP_InfoMenu")]
    bp_info_menu: UObjectRef<InfoMenu>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "UI_OptionMenu")]
    ui_option_menu: UObjectRef<OptionMenu>,

    #[uproperty(BindWidget, BlueprintReadOnly, name = "BP_StartMenu")]
    bp_start_menu: UObjectRef<StartMenu>,

    #[uproperty(BindWidgetAnim, BlueprintReadOnly, name = "Screenshot_Show")]
    screenshot_show: UObjectRef<WidgetAnimation>,

    #[uproperty(BindWidgetAnim, BlueprintReadOnly)]
    show_fullscreen_border: UObjectRef<WidgetAnimation>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "AR")]
    session_config: UObjectRef<ARSessionConfig>,

    #[uproperty(EditAnywhere, BlueprintReadWrite, name = "DefaultUIStyle", category = "Style", default = EUIStyle::Light)]
    default_ui_style: EUIStyle,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Style", default = true)]
    show_tutorial: bool,

    #[uproperty(BlueprintReadOnly, category = "State", default = 0.0)]
    scan_timer: f32,

    #[uproperty(BlueprintReadOnly, category = "State", default = false)]
    session_has_started: bool,

    #[uproperty(BlueprintReadOnly, category = "State", default = false)]
    scanning_is_finished: bool,

    #[uproperty(BlueprintReadOnly, category = "State", default = false)]
    plane_is_selected: bool,

    #[uproperty(BlueprintReadOnly, category = "State", default = false)]
    object_is_placed: bool,

    tutorial_shown: bool,
}

#[uclass_impl]
impl MainMenu {
    #[ufunction(Override)]
    fn on_initialized(&mut self) {
        if let Err(e) = self.init() {
            log::warn!("[AR] main menu init failed: {e}");
        }
    }

    #[ufunction(Override)]
    fn tick(&mut self, _my_geometry: UStructRef<FGeometry>, in_delta_time: f32) {
        self.update(f64::from(in_delta_time));
    }

    #[ufunction]
    fn start_session(&mut self) {
        ARBlueprintLibrary::start_ar_session(self.session_config());
    }

    #[ufunction]
    fn placed(&mut self) {
        show(self.bp_start_menu(), false);
        show(self.bp_bottom_menu(), true);
        self.set_object_is_placed(true);

        if !self.show_tutorial() || self.tutorial_shown() {
            return;
        }

        self.set_tutorial_shown(true);
        let style = OptionMenu::from_obj(self.ui_option_menu())
            .map(|menu| menu.ui_style())
            .unwrap_or(EUIStyle::Light);

        if let Ok(mut bottom) = BottomMenu::from_obj(self.bp_bottom_menu()) {
            bottom.play_tutorial(style);
        }

        self.show_fullscreen_bg();
    }

    #[ufunction]
    fn plane_selected(&mut self, _bp_plane: UObjectRef<Plane>) {
        if let Ok(start) = StartMenu::from_obj(self.bp_start_menu()) {
            start.to_place_object_state();
        }

        self.set_plane_is_selected(true);
    }

    #[ufunction(BlueprintCallable)]
    pub fn take_screenshot(&mut self) {
        let me = self.as_ref();
        show(self.screenshot_overlay(), true);
        play(
            me,
            self.screenshot_show(),
            Some(method(self, Self::screenshot_done)),
        );

        let saved = set_timer(
            &me,
            Duration::from_secs(1),
            false,
            method(self, Self::save_screenshot),
        );

        if let Err(e) = saved {
            log::warn!("[AR] the screenshot timer was not set: {e}");
        }
    }

    #[ufunction]
    fn save_screenshot(&mut self) {
        KismetSystemLibrary::execute_console_command(self.as_ref().upcast_to(), "Shot ", None);
    }

    #[ufunction(name = "ScreenShotDone")]
    fn screenshot_done(&mut self) {
        show(self.screenshot_overlay(), false);
    }

    #[ufunction(BlueprintCallable)]
    pub fn reset_placeable(&mut self) {
        self.set_plane_is_selected(false);
        self.set_object_is_placed(false);
        show(self.bp_start_menu(), true);

        if let Ok(start) = StartMenu::from_obj(self.bp_start_menu()) {
            start.to_select_plane_state();
        }

        show(self.ui_option_menu(), false);
        show(self.bp_bottom_menu(), false);

        if let Some(mut pawn) = self.ar_pawn() {
            pawn.reset_placement();
        }
    }

    #[ufunction(BlueprintCallable)]
    pub fn show_option_menu(&mut self) {
        show(self.ui_option_menu(), true);
        show(self.bp_bottom_menu(), false);
    }

    #[ufunction(BlueprintCallable)]
    pub fn close_options_menu(&mut self) {
        show(self.ui_option_menu(), false);
        show(self.bp_bottom_menu(), true);
    }

    #[ufunction(BlueprintCallable)]
    pub fn toggle_snap(&mut self, snap: bool) {
        if let Some(pawn) = self.ar_pawn() {
            pawn.set_snapping_transform(snap);
        }
    }

    #[ufunction(BlueprintCallable)]
    pub fn toggle_hud(&mut self, hud: bool) {
        if let Some(pawn) = self.ar_pawn() {
            pawn.set_show_hud_interaction(hud);
        }
    }

    #[ufunction(BlueprintCallable)]
    pub fn switch_all_style(&mut self, style: EUIStyle) {
        self.switch_style(style);
    }

    #[ufunction(BlueprintCallable)]
    pub fn show_info(&mut self) {
        show(self.bp_info_menu(), true);
        show(self.ui_option_menu(), false);
        show(self.bp_bottom_menu(), false);
    }

    #[ufunction(BlueprintCallable)]
    pub fn close_info(&mut self) {
        show(self.bp_info_menu(), false);
        show(self.bp_bottom_menu(), true);
    }

    #[ufunction(BlueprintCallable)]
    pub fn switch_style(&mut self, style: EUIStyle) {
        let styled: [UObjectRef<UserWidget>; 4] = [
            self.ui_option_menu().upcast_to(),
            self.bp_bottom_menu().upcast_to(),
            self.bp_info_menu().upcast_to(),
            self.bp_start_menu().upcast_to(),
        ];

        for widget in styled {
            if let Ok(widget) = StylizedUI::from_obj(widget) {
                widget.call_switch_style(style);
            }
        }
    }

    #[ufunction(name = "ShowFullscreenBG")]
    pub fn show_fullscreen_bg(&mut self) {
        show(self.fullscreen_bg(), true);
        play(self.as_ref(), self.show_fullscreen_border(), None);
    }
}

impl MainMenu {
    fn ar_pawn(&self) -> Option<ARPawn> {
        ARPawn::from_obj(GameplayStatics::get_player_pawn(
            self.as_ref().upcast_to(),
            0,
        ))
        .ok()
    }

    fn init(&mut self) -> RustealResult<()> {
        for widget in [
            self.ui_option_menu().upcast_to::<Widget>(),
            self.screenshot_overlay().upcast_to(),
            self.bp_bottom_menu().upcast_to(),
            self.bp_info_menu().upcast_to(),
        ] {
            show(widget, false);
        }

        self.switch_style(self.default_ui_style());
        OptionMenu::from_obj(self.ui_option_menu())?.set_default_style(self.default_ui_style());

        StartMenu::from_obj(self.bp_start_menu())?
            .start_session_add(method(self, Self::start_session))?
            .detach();

        let bottom = BottomMenu::from_obj(self.bp_bottom_menu())?;
        bottom
            .show_info_pressed_add(method(self, Self::show_info))?
            .detach();
        bottom
            .screenshot_pressed_add(method(self, Self::take_screenshot))?
            .detach();
        bottom
            .reset_pressed_add(method(self, Self::reset_placeable))?
            .detach();
        bottom
            .options_pressed_add(method(self, Self::show_option_menu))?
            .detach();

        InfoMenu::from_obj(self.bp_info_menu())?
            .close_info_menu_add(method(self, Self::close_info))?
            .detach();

        let options = OptionMenu::from_obj(self.ui_option_menu())?;
        options
            .close_options_add(method(self, Self::close_options_menu))?
            .detach();
        options
            .toggle_snap_event_add(method(self, Self::toggle_snap))?
            .detach();
        options
            .toggle_hud_event_add(method(self, Self::toggle_hud))?
            .detach();
        options
            .switch_ui_event_add(method(self, Self::switch_all_style))?
            .detach();

        if let Some(pawn) = self.ar_pawn() {
            pawn.ar_object_spawned_add(method(self, Self::placed))?
                .detach();
            pawn.bp_plane_is_selected_add(method(self, Self::plane_selected))?
                .detach();
        }

        Ok(())
    }

    fn update(&mut self, delta: f64) {
        if !self.session_has_started() {
            if ARBlueprintLibrary::get_ar_session_status()
                .as_ref()
                .get_status()
                == EARSessionStatus::Running
            {
                if let Ok(start) = StartMenu::from_obj(self.bp_start_menu()) {
                    start.to_scanning_state();
                }

                self.set_session_has_started(true);
            }

            return;
        }

        if self.scanning_is_finished() {
            return;
        }

        self.set_scan_timer(self.scan_timer() + delta as f32);

        if f64::from(self.scan_timer()) <= model::SCAN_SECONDS
            || ARBlueprintLibrary::get_all_geometries().is_empty()
        {
            return;
        }

        if let Ok(start) = StartMenu::from_obj(self.bp_start_menu()) {
            start.to_select_plane_state();
        }

        self.set_scanning_is_finished(true);

        if let Some(pawn) = self.ar_pawn() {
            pawn.set_scanning_is_done(true);
        }
    }
}

#[uclass(parent = UserWidget)]
pub struct InteractionHUD {
    #[uproperty(BindWidget, BlueprintReadOnly)]
    interaction_text: UObjectRef<TextBlock>,

    #[uproperty(BindWidget, BlueprintReadOnly)]
    interaction_unit: UObjectRef<TextBlock>,

    #[uproperty(BindWidget, BlueprintReadOnly)]
    interaction_icon: UObjectRef<Image>,
}

#[uclass_impl]
impl InteractionHUD {}

impl InteractionHUD {
    pub fn show(&self, text: &str, unit: &str) {
        if let Ok(label) = self.interaction_text().checked() {
            label.set_text(text);
        }

        if let Ok(label) = self.interaction_unit().checked() {
            label.set_text(unit);
        }
    }

    pub fn set_icon(&self, icon: UObjectRef<Texture2D>) {
        if let Ok(image) = self.interaction_icon().checked() {
            image.set_brush_from_texture(icon, Some(true));
        }
    }
}
