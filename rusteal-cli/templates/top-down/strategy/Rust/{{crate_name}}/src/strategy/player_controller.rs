use bindings::engine::{
    ActorExt, CameraComponentExt, ControllerExt, EDrawDebugTrace, EObjectTypeQuery,
    ETraceTypeQuery, FHitResultExt, GameplayStatics, KismetSystemLibrary, Pawn, PawnExt,
    PlayerController, PlayerControllerExt,
};
use bindings::enhanced_input::{
    ETriggerEvent, EnhancedInputLocalPlayerSubsystemExt, FInputActionValue, InputAction,
    InputMappingContext,
};
use bindings::input_core::ETouchIndex;
use bindings::prelude::*;
use bindings::umg::UserWidgetExt;
use glam::{DQuat, DVec2, DVec3};
use rusteal_runtime::runtime::input::should_display_touch_interface;
use rusteal_runtime::runtime::{
    LOG_ERROR, LOG_WARNING, OwnedStruct, Rotator, RustealResult, SubclassOf, UObjectRef,
    UStructRef, ulog,
};
use rusteal_runtime::{uclass, uclass_impl};

use super::hud::StrategyHUD;
use super::model::{self, ZoomRange};
use super::pawn::StrategyPawn;
use super::touch_controls::StrategyTouchControls;
use super::unit::StrategyUnit;

const RECENTLY_RENDERED_TOLERANCE: f32 = 0.2;
const TAP_AFTER_DRAG_SCROLL_DELAY: f64 = 0.1;
const TOUCH_TRACE_DISTANCE: f64 = 10_000.0;
const TOUCH_PROJECTION_DISTANCE: f64 = 100_000.0;

#[uclass(parent = PlayerController)]
pub struct StrategyPlayerController {
    #[uproperty(EditAnywhere, category = "Input")]
    mouse_mapping_context: UObjectRef<InputMappingContext>,

    #[uproperty(EditAnywhere, category = "Input")]
    touch_mapping_context: UObjectRef<InputMappingContext>,

    #[uproperty(EditAnywhere, category = "Input")]
    move_camera_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    zoom_camera_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    reset_camera_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    select_click_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    select_click_additive_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    select_all_double_click_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    select_hold_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    interact_click_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    interact_hold_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    selection_modifier_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    touch_primary_hold_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    touch_secondary_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    mobile_controls_widget_class: SubclassOf<StrategyTouchControls>,

    #[uproperty(EditAnywhere, category = "Input", default = false)]
    b_force_touch_controls: bool,

    #[uproperty(EditAnywhere, category = "Input", default = 250.0)]
    selection_radius: f32,

    #[uproperty(EditAnywhere, category = "Input", default = 0.15)]
    touch_drag_scroll_hold_time: f32,

    #[uproperty(EditAnywhere, category = "Camera", default = 1000.0)]
    min_zoom_level: f32,

    #[uproperty(EditAnywhere, category = "Camera", default = 2500.0)]
    max_zoom_level: f32,

    #[uproperty(EditAnywhere, category = "Camera", default = 100.0)]
    zoom_scaling: f32,

    #[uproperty(EditAnywhere, category = "Camera", default = 0.1)]
    drag_multiplier: f32,

    #[uproperty(EditAnywhere, category = "Selection")]
    selection_trace_channel: ETraceTypeQuery,

    controlled_camera_pawn: UObjectRef<StrategyPawn>,

    strategy_hud: UObjectRef<StrategyHUD>,

    mobile_controls_widget: UObjectRef<StrategyTouchControls>,

    starting_drag_scroll_position: DVec2,

    starting_box_selection_position: DVec2,

    touch_hold_start_time: f64,

    last_touch_drag_scroll_time: f64,

    camera_zoom: f32,

    default_zoom: f32,

    controlled_units: Vec<UObjectRef<StrategyUnit>>,
}

#[uclass_impl]
impl StrategyPlayerController {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        self.as_ref().checked()?.set_show_mouse_cursor(true);

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let Err(e) = self.setup_input() {
            ulog!(LOG_WARNING, "[Strategy] input setup failed: {e}");
        }
    }

    #[ufunction(Override)]
    fn receive_possess(&mut self, possessed_pawn: UObjectRef<Pawn>) {
        let Ok(camera_pawn) = StrategyPawn::from_obj(possessed_pawn) else {
            ulog!(
                LOG_ERROR,
                "[Strategy] the possessed pawn is not a StrategyPawn"
            );

            return;
        };

        self.set_controlled_camera_pawn(possessed_pawn.cast().unwrap_or_default());

        if let Ok(camera) = camera_pawn.get_camera().and_then(|camera| camera.checked()) {
            let ortho_width = camera.get_ortho_width();
            self.set_default_zoom(ortho_width);
            self.set_camera_zoom(ortho_width);
        }

        if let Ok(me) = self.as_ref().checked() {
            self.set_strategy_hud(me.get_hud().cast().unwrap_or_default());
        }

        if let Ok(mobile_controls) = StrategyTouchControls::from_obj(self.mobile_controls_widget())
        {
            mobile_controls.bp_set_zoom_percentage(self.get_default_zoom_percentage());
        }
    }

    #[ufunction]
    fn move_camera(&mut self, value: UStructRef<FInputActionValue>) {
        let (Ok(me), Ok(pawn)) = (
            self.as_ref().checked(),
            self.controlled_camera_pawn().checked(),
        ) else {
            return;
        };

        let (forward_scale, right_scale) = model::camera_move_scales(value.axis2d());

        let control_rotation = me.get_control_rotation().to_rotator();
        let forward_rotation = Rotator::new(0.0, control_rotation.yaw, control_rotation.roll);

        let right_rotation = Rotator::new(0.0, control_rotation.yaw, 0.0);

        let forward = DQuat::from(forward_rotation) * DVec3::X;

        pawn.add_movement_input(
            &FVector::from_dvec3(forward),
            Some(forward_scale as f32),
            Some(false),
        );

        let right = DQuat::from(right_rotation) * DVec3::Y;

        pawn.add_movement_input(
            &FVector::from_dvec3(right),
            Some(right_scale as f32),
            Some(false),
        );
    }

    #[ufunction]
    fn zoom_camera(&mut self, value: UStructRef<FInputActionValue>) {
        self.do_camera_modify_zoom_command(value.axis1d() as f32 * self.zoom_scaling());
    }

    #[ufunction]
    fn reset_camera(&mut self) {
        self.do_camera_reset_zoom_command();
    }

    #[ufunction]
    fn select_hold_started(&mut self) {
        self.set_starting_box_selection_position(self.get_mouse_location_for_player());
    }

    #[ufunction]
    fn select_hold_triggered(&mut self) {
        let selection_position = self.get_mouse_location_for_player();
        self.update_selection_box(selection_position);
    }

    #[ufunction]
    fn select_hold_completed(&mut self) {
        self.hide_selection_box();
    }

    #[ufunction]
    fn select_click(&mut self) {
        if let Some(cursor_location) = self.get_location_under_cursor() {
            self.do_select_command(cursor_location, false);
        }
    }

    #[ufunction]
    fn select_click_additive(&mut self) {
        if let Some(cursor_location) = self.get_location_under_cursor() {
            self.do_select_command(cursor_location, true);
        }
    }

    #[ufunction]
    fn select_all_double_click(&mut self) {
        self.do_select_all_units_on_screen_command();
    }

    #[ufunction]
    fn interact_hold_started(&mut self) {
        self.set_starting_drag_scroll_position(self.get_mouse_location_for_player());
    }

    #[ufunction]
    fn interact_hold_triggered(&mut self) {
        self.do_camera_drag_scroll_command(self.get_mouse_location_for_player());
    }

    #[ufunction]
    fn interact_click(&mut self) {
        if let Some(cursor_location) = self.get_location_under_cursor() {
            self.do_move_units_command(cursor_location);
        }
    }

    #[ufunction]
    fn touch_primary_hold_started(&mut self, value: UStructRef<FInputActionValue>) {
        self.set_starting_drag_scroll_position(value.axis2d());
        self.set_touch_hold_start_time(self.game_time());
    }

    #[ufunction]
    fn touch_primary_hold_triggered(&mut self, value: UStructRef<FInputActionValue>) {
        let input_vector = value.axis2d();

        self.set_starting_box_selection_position(input_vector);

        let elapsed_time = self.game_time() - self.touch_hold_start_time();

        if elapsed_time > f64::from(self.touch_drag_scroll_hold_time()) {
            self.do_camera_drag_scroll_command(input_vector);

            self.set_last_touch_drag_scroll_time(self.game_time());
        }
    }

    #[ufunction]
    fn touch_primary_hold_completed(&mut self) {
        if self.game_time() - self.last_touch_drag_scroll_time() > TAP_AFTER_DRAG_SCROLL_DELAY {
            let touch_location = self.project_touch_point_to_world_space();

            if !self.do_select_command(touch_location, true) {
                self.do_move_units_command(touch_location);
            }
        }
    }

    #[ufunction]
    fn touch_secondary_triggered(&mut self, value: UStructRef<FInputActionValue>) {
        self.update_selection_box(value.axis2d());
    }

    #[ufunction]
    fn touch_secondary_completed(&mut self) {
        self.hide_selection_box();
    }

    #[ufunction(BlueprintImplementableEvent, name = "BP_CursorFeedback")]
    fn bp_cursor_feedback(&self, location: &OwnedStruct<FVector>, b_positive: bool) {}
}

impl StrategyPlayerController {
    pub fn drag_select_units(&mut self, units: Vec<UObjectRef<StrategyUnit>>) {
        if !units.is_empty() {
            self.do_deselect_all_units_command();

            for unit in units {
                self.select(unit);
            }
        } else if !self.controlled_units().is_empty() {
            self.do_deselect_all_units_command();
        }
    }

    pub fn get_selected_units(&self) -> Vec<UObjectRef<StrategyUnit>> {
        self.controlled_units()
    }

    pub fn get_default_zoom_percentage(&self) -> f32 {
        self.zoom_range().default_percentage()
    }

    pub fn do_select_command(
        &mut self,
        select_location: DVec3,
        b_additive_selection: bool,
    ) -> bool {
        if !b_additive_selection {
            self.do_deselect_all_units_command();
        }

        let me = self.as_ref();

        let (_, overlaps) = KismetSystemLibrary::sphere_overlap_actors(
            me.upcast_to(),
            &FVector::from_dvec3(select_location),
            self.selection_radius(),
            &[EObjectTypeQuery::ObjectTypeQuery3],
            SubclassOf::<StrategyUnit>::base().upcast_to(),
            &[],
        );

        let Some(unit) = overlaps
            .into_iter()
            .find_map(|actor| actor.cast::<StrategyUnit>().ok())
        else {
            return false;
        };

        if self.controlled_units().contains(&unit) {
            self.controlled_units_mut()
                .retain(|selected| *selected != unit);

            if let Ok(unit) = StrategyUnit::from_obj(unit) {
                unit.unit_deselected();
            }
        } else {
            self.select(unit);
        }

        true
    }

    pub fn do_select_all_units_on_screen_command(&mut self) {
        let units = GameplayStatics::get_all_actors_of_class(
            self.as_ref().upcast_to(),
            SubclassOf::<StrategyUnit>::base().upcast_to(),
        );

        for unit in units
            .into_iter()
            .filter_map(|actor| actor.cast::<StrategyUnit>().ok())
        {
            let on_screen = unit
                .checked()
                .is_ok_and(|actor| actor.was_recently_rendered(Some(RECENTLY_RENDERED_TOLERANCE)));

            if !self.controlled_units().contains(&unit) && on_screen {
                self.select(unit);
            }
        }
    }

    pub fn do_deselect_all_units_command(&mut self) {
        for unit in self.controlled_units() {
            if let Ok(unit) = StrategyUnit::from_obj(unit) {
                unit.unit_deselected();
            }
        }

        self.controlled_units_mut().clear();
    }

    pub fn do_toggle_select_all_units_command(&mut self) {
        if !self.controlled_units().is_empty() {
            self.do_deselect_all_units_command();
        } else {
            self.do_select_all_units_on_screen_command();
        }
    }

    pub fn do_camera_drag_scroll_command(&mut self, current_cursor_position: DVec2) {
        let offset = model::drag_scroll_offset(
            self.starting_drag_scroll_position(),
            current_cursor_position,
            self.drag_multiplier(),
        );

        if let Ok(pawn) = self.controlled_camera_pawn().checked() {
            pawn.k2_add_actor_world_offset(&FVector::from_dvec3(offset), false, false);
        }
    }

    pub fn do_move_units_command(&mut self, goal_location: DVec3) {
        let units = self.controlled_units();

        if units.is_empty() {
            self.bp_cursor_feedback(&FVector::from_dvec3(goal_location), false);
            return;
        }

        let closest_unit = self.get_closest_selected_unit_to_location(goal_location);

        for unit in &units {
            if let Ok(mut strategy_unit) = StrategyUnit::from_obj(*unit) {
                strategy_unit.move_to_location(
                    goal_location,
                    Some(*unit) == closest_unit,
                    units.clone(),
                );
            }
        }

        self.bp_cursor_feedback(&FVector::from_dvec3(goal_location), true);
    }

    pub fn do_camera_modify_zoom_command(&mut self, zoom_delta: f32) {
        let zoom = self.zoom_range().modified(self.camera_zoom(), zoom_delta);
        self.apply_zoom(zoom);
    }

    pub fn do_camera_reset_zoom_command(&mut self) {
        self.apply_zoom(self.default_zoom());
    }

    pub fn do_camera_set_zoom_percentage_command(&mut self, percentage: f32) {
        let zoom = self.zoom_range().at_percentage(percentage);
        self.apply_zoom(zoom);
    }

    fn setup_input(&mut self) -> RustealResult<()> {
        let me: UObjectRef<PlayerController> = self.as_ref();

        if !me.checked()?.is_local_player_controller() {
            return Ok(());
        }

        let chosen_context = if self.should_use_touch_controls() {
            self.touch_mapping_context()
        } else {
            self.mouse_mapping_context()
        };

        enhanced_input_subsystem(me)?
            .checked()?
            .add_mapping_context(chosen_context, 0, &OwnedStruct::new());

        bind_action(
            &me,
            self.move_camera_action(),
            ETriggerEvent::Triggered,
            "MoveCamera",
        )?;

        bind_action(
            &me,
            self.zoom_camera_action(),
            ETriggerEvent::Triggered,
            "ZoomCamera",
        )?;

        bind_action(
            &me,
            self.reset_camera_action(),
            ETriggerEvent::Triggered,
            "ResetCamera",
        )?;

        let select_hold = self.select_hold_action();

        bind_action(
            &me,
            select_hold,
            ETriggerEvent::Started,
            "SelectHoldStarted",
        )?;

        bind_action(
            &me,
            select_hold,
            ETriggerEvent::Triggered,
            "SelectHoldTriggered",
        )?;

        bind_action(
            &me,
            select_hold,
            ETriggerEvent::Completed,
            "SelectHoldCompleted",
        )?;

        bind_action(
            &me,
            select_hold,
            ETriggerEvent::Canceled,
            "SelectHoldCompleted",
        )?;

        bind_action(
            &me,
            self.select_click_action(),
            ETriggerEvent::Completed,
            "SelectClick",
        )?;

        bind_action(
            &me,
            self.select_click_additive_action(),
            ETriggerEvent::Completed,
            "SelectClickAdditive",
        )?;

        bind_action(
            &me,
            self.select_all_double_click_action(),
            ETriggerEvent::Completed,
            "SelectAllDoubleClick",
        )?;

        let interact_hold = self.interact_hold_action();

        bind_action(
            &me,
            interact_hold,
            ETriggerEvent::Started,
            "InteractHoldStarted",
        )?;

        bind_action(
            &me,
            interact_hold,
            ETriggerEvent::Triggered,
            "InteractHoldTriggered",
        )?;

        bind_action(
            &me,
            self.interact_click_action(),
            ETriggerEvent::Completed,
            "InteractClick",
        )?;

        let touch_primary = self.touch_primary_hold_action();

        bind_action(
            &me,
            touch_primary,
            ETriggerEvent::Started,
            "TouchPrimaryHoldStarted",
        )?;

        bind_action(
            &me,
            touch_primary,
            ETriggerEvent::Triggered,
            "TouchPrimaryHoldTriggered",
        )?;

        bind_action(
            &me,
            touch_primary,
            ETriggerEvent::Completed,
            "TouchPrimaryHoldCompleted",
        )?;

        let touch_secondary = self.touch_secondary_action();

        bind_action(
            &me,
            touch_secondary,
            ETriggerEvent::Triggered,
            "TouchSecondaryTriggered",
        )?;

        bind_action(
            &me,
            touch_secondary,
            ETriggerEvent::Completed,
            "TouchSecondaryCompleted",
        )?;

        bind_action(
            &me,
            touch_secondary,
            ETriggerEvent::Canceled,
            "TouchSecondaryCompleted",
        )?;

        if self.should_use_touch_controls() {
            match create_widget_of_class(&me, self.mobile_controls_widget_class()) {
                Ok(widget) => {
                    widget.checked()?.add_to_player_screen(Some(0));

                    let mut mobile_controls = StrategyTouchControls::from_obj(widget)?;
                    mobile_controls.set_player_controller(self.as_ref().cast()?);
                    self.set_mobile_controls_widget(widget);
                }
                Err(_) => ulog!(LOG_ERROR, "Could not spawn mobile controls widget."),
            }
        }

        Ok(())
    }

    fn should_use_touch_controls(&self) -> bool {
        should_display_touch_interface() || self.b_force_touch_controls()
    }

    fn select(&mut self, unit: UObjectRef<StrategyUnit>) {
        self.controlled_units_mut().push(unit);

        if let Ok(unit) = StrategyUnit::from_obj(unit) {
            unit.unit_selected();
        }
    }

    fn update_selection_box(&mut self, selection_position: DVec2) {
        let start = self.starting_box_selection_position();
        let selection_size = selection_position - start;

        if let Ok(mut hud) = StrategyHUD::from_obj(self.strategy_hud()) {
            hud.drag_select_update(start, selection_size, selection_position, true);
        }
    }

    fn hide_selection_box(&mut self) {
        if let Ok(mut hud) = StrategyHUD::from_obj(self.strategy_hud()) {
            hud.drag_select_update(DVec2::ZERO, DVec2::ZERO, DVec2::ZERO, false);
        }
    }

    fn apply_zoom(&mut self, zoom: f32) {
        self.set_camera_zoom(zoom);

        if let Ok(pawn) = StrategyPawn::from_obj(self.controlled_camera_pawn()) {
            let _ = pawn.set_zoom_modifier(zoom);
        }
    }

    fn zoom_range(&self) -> ZoomRange {
        ZoomRange {
            min: self.min_zoom_level(),
            max: self.max_zoom_level(),
            default: self.default_zoom(),
        }
    }

    fn game_time(&self) -> f64 {
        GameplayStatics::get_time_seconds(self.as_ref().upcast_to())
    }

    fn get_closest_selected_unit_to_location(
        &self,
        target_location: DVec3,
    ) -> Option<UObjectRef<StrategyUnit>> {
        let (units, locations): (Vec<_>, Vec<_>) = self
            .controlled_units()
            .into_iter()
            .filter_map(|unit| {
                Some((
                    unit,
                    unit.checked().ok()?.k2_get_actor_location().to_dvec3(),
                ))
            })
            .unzip();

        model::closest_to(&locations, target_location).map(|index| units[index])
    }

    fn get_mouse_location_for_player(&self) -> DVec2 {
        match self.as_ref().checked().map(|me| me.get_mouse_position()) {
            Ok((true, mouse_x, mouse_y)) => DVec2::new(f64::from(mouse_x), f64::from(mouse_y)),
            _ => DVec2::ZERO,
        }
    }

    fn get_location_under_cursor(&self) -> Option<DVec3> {
        let me = self.as_ref().checked().ok()?;

        let (blocking_hit, out_hit) =
            me.get_hit_result_under_cursor_by_channel(self.selection_trace_channel(), false);

        blocking_hit.then(|| out_hit.as_ref().get_location().to_dvec3())
    }

    fn project_touch_point_to_world_space(&self) -> DVec3 {
        let Ok(me) = self.as_ref().checked() else {
            return DVec3::ZERO;
        };

        let (touch_x, touch_y, _pressed) = me.get_input_touch_state(ETouchIndex::Touch1);

        let (deprojected, world_location, world_direction) =
            me.deproject_screen_position_to_world(touch_x, touch_y);

        if !deprojected {
            return DVec3::ZERO;
        }

        let (world_location, world_direction) =
            (world_location.to_dvec3(), world_direction.to_dvec3());

        let (hit, out_hit) = KismetSystemLibrary::line_trace_single(
            me.as_ref().upcast_to(),
            &FVector::from_dvec3(world_location),
            &FVector::from_dvec3(world_location + world_direction * TOUCH_TRACE_DISTANCE),
            ETraceTypeQuery::TraceTypeQuery1,
            false,
            &[],
            EDrawDebugTrace::None,
            false,
            &OwnedStruct::new(),
            &OwnedStruct::new(),
            None,
        );

        if hit {
            return out_hit.as_ref().get_impact_point().to_dvec3();
        }

        model::line_plane_intersection(
            world_location,
            world_location + world_direction * TOUCH_PROJECTION_DISTANCE,
            DVec3::ZERO,
            DVec3::Z,
        )
    }
}
