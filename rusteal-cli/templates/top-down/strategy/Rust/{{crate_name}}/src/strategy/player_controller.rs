// StrategyPlayerController: the Strategy variant's `AStrategyPlayerController`
// in Rust. Handles unit selection and commands, with both mouse and touch
// controls: clicks, double clicks and drawn boxes select units, clicks and
// taps send them somewhere, and the camera pans, drags and zooms.
//
// `SetupInputComponent` and `OnPossess` are C++ virtuals; here their work
// happens in `ReceiveBeginPlay` and `ReceivePossess`. The C++ touch handler
// reads how long the touch has been held from the input action instance,
// which a Rust handler does not receive; the controller times the hold from
// its start instead. The Blueprint child `BP_StrategyPlayerController` gives
// it the mapping contexts, the input actions and the mobile controls widget,
// and spawns the cursor feedback effect.

use bindings::engine::{
    ActorExt, CameraComponentExt, ControllerExt, EDrawDebugTrace, EObjectTypeQuery, ETraceTypeQuery,
    FHitResultExt, GameplayStatics, KismetSystemLibrary, Pawn, PawnExt, PlayerController,
    PlayerControllerExt,
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

/// How long a unit may go unrendered and still count as on screen for a
/// select all.
const RECENTLY_RENDERED_TOLERANCE: f32 = 0.2;
/// A tap right after a drag scroll ends is ignored for this long, in seconds.
const TAP_AFTER_DRAG_SCROLL_DELAY: f64 = 0.1;
/// How far a touch is traced into the level.
const TOUCH_TRACE_DISTANCE: f64 = 10_000.0;
/// How far a touch that hits nothing is projected onto the ground plane.
const TOUCH_PROJECTION_DISTANCE: f64 = 100_000.0;

/// Player Controller for a top-down strategy game. Handles unit selection
/// and commands. Implements both mouse and touch controls.
#[uclass(parent = PlayerController)]
pub struct StrategyPlayerController {
    /// Input mapping context to use with mouse input
    #[uproperty(EditAnywhere, category = "Input")]
    mouse_mapping_context: UObjectRef<InputMappingContext>,

    /// Input mapping context to use with touch input
    #[uproperty(EditAnywhere, category = "Input")]
    touch_mapping_context: UObjectRef<InputMappingContext>,

    /// Input Action for moving the camera
    #[uproperty(EditAnywhere, category = "Input")]
    move_camera_action: UObjectRef<InputAction>,

    /// Input Action for zooming the camera
    #[uproperty(EditAnywhere, category = "Input")]
    zoom_camera_action: UObjectRef<InputAction>,

    /// Input Action for resetting the camera to its default position
    #[uproperty(EditAnywhere, category = "Input")]
    reset_camera_action: UObjectRef<InputAction>,

    /// Input Action for select click
    #[uproperty(EditAnywhere, category = "Input")]
    select_click_action: UObjectRef<InputAction>,

    /// Input Action for additive select click
    #[uproperty(EditAnywhere, category = "Input")]
    select_click_additive_action: UObjectRef<InputAction>,

    /// Input Action for select all double click
    #[uproperty(EditAnywhere, category = "Input")]
    select_all_double_click_action: UObjectRef<InputAction>,

    /// Input Action for select press and hold
    #[uproperty(EditAnywhere, category = "Input")]
    select_hold_action: UObjectRef<InputAction>,

    /// Input Action for click interaction
    #[uproperty(EditAnywhere, category = "Input")]
    interact_click_action: UObjectRef<InputAction>,

    /// Input Action for interaction press and hold
    #[uproperty(EditAnywhere, category = "Input")]
    interact_hold_action: UObjectRef<InputAction>,

    /// Input Action for modifying selection mode
    #[uproperty(EditAnywhere, category = "Input")]
    selection_modifier_action: UObjectRef<InputAction>,

    /// Input Action for primary touch hold
    #[uproperty(EditAnywhere, category = "Input")]
    touch_primary_hold_action: UObjectRef<InputAction>,

    /// Input Action for secondary touch
    #[uproperty(EditAnywhere, category = "Input")]
    touch_secondary_action: UObjectRef<InputAction>,

    /// Touch controls widget class to spawn on mobile platforms
    #[uproperty(EditAnywhere, category = "Input")]
    mobile_controls_widget_class: SubclassOf<StrategyTouchControls>,

    /// If true, the PC will be initialized with touchscreen controls
    #[uproperty(EditAnywhere, category = "Input", default = false)]
    b_force_touch_controls: bool,

    /// Max distance to look for nearby units when doing a click or touch interaction
    #[uproperty(EditAnywhere, category = "Input", default = 250.0)]
    selection_radius: f32,

    /// Time the finger needs to be held down to initiate a drag scroll
    #[uproperty(EditAnywhere, category = "Input", default = 0.15)]
    touch_drag_scroll_hold_time: f32,

    /// Minimum allowed camera zoom level
    #[uproperty(EditAnywhere, category = "Camera", default = 1000.0)]
    min_zoom_level: f32,

    /// Maximum allowed camera zoom level
    #[uproperty(EditAnywhere, category = "Camera", default = 2500.0)]
    max_zoom_level: f32,

    /// Scales zoom inputs by this value
    #[uproperty(EditAnywhere, category = "Camera", default = 100.0)]
    zoom_scaling: f32,

    /// Affects how fast the camera moves while dragging with the mouse
    #[uproperty(EditAnywhere, category = "Camera", default = 0.1)]
    drag_multiplier: f32,

    /// Trace channel to use for selection trace checks
    #[uproperty(EditAnywhere, category = "Selection")]
    selection_trace_channel: ETraceTypeQuery,

    /// Strategy Pawn associated with this controller
    controlled_camera_pawn: UObjectRef<StrategyPawn>,

    /// Strategy HUD associated with this controller
    strategy_hud: UObjectRef<StrategyHUD>,

    /// Pointer to the mobile controls widget
    mobile_controls_widget: UObjectRef<StrategyTouchControls>,

    /// Cached starting position for camera drag scrolling
    starting_drag_scroll_position: DVec2,

    /// Cached starting position for box select
    starting_box_selection_position: DVec2,

    /// Game time when the primary touch hold started
    touch_hold_start_time: f64,

    /// Last game time when a drag scroll was performed, so we can avoid spamming commands on drag scroll end
    last_touch_drag_scroll_time: f64,

    /// Current camera zoom level
    camera_zoom: f32,

    /// Default camera zoom level
    default_zoom: f32,

    /// Currently selected unit list
    controlled_units: Vec<UObjectRef<StrategyUnit>>,
}

#[uclass_impl]
impl StrategyPlayerController {
    /// Everything `AStrategyPlayerController::AStrategyPlayerController()` sets.
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        // mouse cursor should always be shown
        self.as_ref().checked()?.set_show_mouse_cursor(true);
        Ok(())
    }

    /// Initialize input bindings and the touch controls
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let Err(e) = self.setup_input() {
            ulog!(LOG_WARNING, "[Strategy] input setup failed: {e}");
        }
    }

    /// Pawn initialization
    #[ufunction(Override)]
    fn receive_possess(&mut self, possessed_pawn: UObjectRef<Pawn>) {
        // ensure we have the right pawn type
        let Ok(camera_pawn) = StrategyPawn::from_obj(possessed_pawn) else {
            ulog!(LOG_ERROR, "[Strategy] the possessed pawn is not a StrategyPawn");
            return;
        };
        self.set_controlled_camera_pawn(possessed_pawn.cast().unwrap_or_default());

        // set the zoom level from the pawn's camera
        if let Ok(camera) = camera_pawn.get_camera().and_then(|camera| camera.checked()) {
            let ortho_width = camera.get_ortho_width();
            self.set_default_zoom(ortho_width);
            self.set_camera_zoom(ortho_width);
        }

        // cast the HUD pointer
        if let Ok(me) = self.as_ref().checked() {
            self.set_strategy_hud(me.get_hud().cast().unwrap_or_default());
        }

        // if we have a touch controls widget, sync the camera zoom
        if let Ok(mobile_controls) = StrategyTouchControls::from_obj(self.mobile_controls_widget()) {
            mobile_controls.bp_set_zoom_percentage(self.get_default_zoom_percentage());
        }
    }

    // mouse + keyboard input

    /// Moves the camera by the given input
    #[ufunction]
    fn move_camera(&mut self, value: UStructRef<FInputActionValue>) {
        let (Ok(me), Ok(pawn)) = (self.as_ref().checked(), self.controlled_camera_pawn().checked()) else {
            return;
        };
        let (forward_scale, right_scale) = model::camera_move_scales(value.axis2d());

        // get the forward input component vector
        let control_rotation = me.get_control_rotation().to_rotator();
        let forward_rotation = Rotator::new(0.0, control_rotation.yaw, control_rotation.roll);

        // get the right input component vector
        let right_rotation = Rotator::new(0.0, control_rotation.yaw, 0.0);

        // add the forward input
        let forward = DQuat::from(forward_rotation) * DVec3::X;
        pawn.add_movement_input(&FVector::from_dvec3(forward), Some(forward_scale as f32), Some(false));

        // add the right input
        let right = DQuat::from(right_rotation) * DVec3::Y;
        pawn.add_movement_input(&FVector::from_dvec3(right), Some(right_scale as f32), Some(false));
    }

    /// Changes the camera zoom level by the given input
    #[ufunction]
    fn zoom_camera(&mut self, value: UStructRef<FInputActionValue>) {
        self.do_camera_modify_zoom_command(value.axis1d() as f32 * self.zoom_scaling());
    }

    /// Resets the camera to its initial value
    #[ufunction]
    fn reset_camera(&mut self) {
        self.do_camera_reset_zoom_command();
    }

    /// Start a select and hold input
    #[ufunction]
    fn select_hold_started(&mut self) {
        // save the box selection start position
        self.set_starting_box_selection_position(self.get_mouse_location_for_player());
    }

    /// Select and hold input triggered
    #[ufunction]
    fn select_hold_triggered(&mut self) {
        // get the current mouse position
        let selection_position = self.get_mouse_location_for_player();
        self.update_selection_box(selection_position);
    }

    /// Select and hold input completed
    #[ufunction]
    fn select_hold_completed(&mut self) {
        // reset the drag box on the HUD
        self.hide_selection_box();
    }

    /// Select click action
    #[ufunction]
    fn select_click(&mut self) {
        // select at the cursor
        if let Some(cursor_location) = self.get_location_under_cursor() {
            self.do_select_command(cursor_location, false);
        }
    }

    /// Select Click Additive Action
    #[ufunction]
    fn select_click_additive(&mut self) {
        // additive select at the cursor
        if let Some(cursor_location) = self.get_location_under_cursor() {
            self.do_select_command(cursor_location, true);
        }
    }

    /// Select All Double Click Action
    #[ufunction]
    fn select_all_double_click(&mut self) {
        self.do_select_all_units_on_screen_command();
    }

    /// Starts an interaction hold input
    #[ufunction]
    fn interact_hold_started(&mut self) {
        // save the starting interaction position
        self.set_starting_drag_scroll_position(self.get_mouse_location_for_player());
    }

    /// Interaction hold input triggered
    #[ufunction]
    fn interact_hold_triggered(&mut self) {
        // do a drag scroll
        self.do_camera_drag_scroll_command(self.get_mouse_location_for_player());
    }

    /// Interaction click input started
    #[ufunction]
    fn interact_click(&mut self) {
        // do we have a valid interaction location under the cursor?
        if let Some(cursor_location) = self.get_location_under_cursor() {
            // move the selected units to the target location
            self.do_move_units_command(cursor_location);
        }
    }

    // touch input

    /// Touch primary finger hold started
    #[ufunction]
    fn touch_primary_hold_started(&mut self, value: UStructRef<FInputActionValue>) {
        // save the camera drag screen coords and when the hold started
        self.set_starting_drag_scroll_position(value.axis2d());
        self.set_touch_hold_start_time(self.game_time());
    }

    /// Touch primary finger hold triggered
    #[ufunction]
    fn touch_primary_hold_triggered(&mut self, value: UStructRef<FInputActionValue>) {
        let input_vector = value.axis2d();

        // update the box select start position
        self.set_starting_box_selection_position(input_vector);

        let elapsed_time = self.game_time() - self.touch_hold_start_time();
        if elapsed_time > f64::from(self.touch_drag_scroll_hold_time()) {
            self.do_camera_drag_scroll_command(input_vector);

            // save the game time
            self.set_last_touch_drag_scroll_time(self.game_time());
        }
    }

    /// Touch primary finger hold completed
    #[ufunction]
    fn touch_primary_hold_completed(&mut self) {
        // ensure we don't trigger a tap input right after we finish a drag scroll
        if self.game_time() - self.last_touch_drag_scroll_time() > TAP_AFTER_DRAG_SCROLL_DELAY {
            // get the touch location in world space
            let touch_location = self.project_touch_point_to_world_space();

            // try to do a select command
            if !self.do_select_command(touch_location, true) {
                // if nothing was selected, do a move units command instead
                self.do_move_units_command(touch_location);
            }
        }
    }

    /// Touch secondary finger triggered
    #[ufunction]
    fn touch_secondary_triggered(&mut self, value: UStructRef<FInputActionValue>) {
        // get the touch 2 screen coords
        self.update_selection_box(value.axis2d());
    }

    /// Touch secondary finger completed
    #[ufunction]
    fn touch_secondary_completed(&mut self) {
        // hide the selection box
        self.hide_selection_box();
    }

    /// Spawns the positive cursor effect
    #[ufunction(BlueprintImplementableEvent, name = "BP_CursorFeedback")]
    fn bp_cursor_feedback(&self, location: &OwnedStruct<FVector>, b_positive: bool) {}
}

impl StrategyPlayerController {
    /// Updates selected units from the HUD's drag select box
    pub fn drag_select_units(&mut self, units: Vec<UObjectRef<StrategyUnit>>) {
        // do we have units in the list?
        if !units.is_empty() {
            // ensure any previous units are deselected
            self.do_deselect_all_units_command();

            // select each new unit
            for unit in units {
                // add the unit to the selection list and select it
                self.select(unit);
            }
        } else if !self.controlled_units().is_empty() {
            // release any currently selected units since nothing is on the box
            self.do_deselect_all_units_command();
        }
    }

    /// Passes the list of selected units
    pub fn get_selected_units(&self) -> Vec<UObjectRef<StrategyUnit>> {
        self.controlled_units()
    }

    /// Returns the default camera zoom percentage value
    pub fn get_default_zoom_percentage(&self) -> f32 {
        self.zoom_range().default_percentage()
    }

    // commands

    /// Attempt to select or deselect a unit near the given location. Supports additive selection modifier.
    pub fn do_select_command(&mut self, select_location: DVec3, b_additive_selection: bool) -> bool {
        // deselect any units unless this is an additive selection
        if !b_additive_selection {
            self.do_deselect_all_units_command();
        }

        // do an overlap test at the cursor location
        let me = self.as_ref();
        let (_, overlaps) = KismetSystemLibrary::sphere_overlap_actors(
            me.upcast_to(),
            &FVector::from_dvec3(select_location),
            self.selection_radius(),
            &[EObjectTypeQuery::ObjectTypeQuery3], // Pawn
            SubclassOf::<StrategyUnit>::base().upcast_to(),
            &[],
        );

        // find the first unit we've overlapped
        let Some(unit) = overlaps.into_iter().find_map(|actor| actor.cast::<StrategyUnit>().ok()) else {
            // didn't find a unit
            return false;
        };

        // is this unit already selected?
        if self.controlled_units().contains(&unit) {
            // deselect the unit
            self.controlled_units_mut().retain(|selected| *selected != unit);
            if let Ok(unit) = StrategyUnit::from_obj(unit) {
                unit.unit_deselected();
            }
        } else {
            // select the unit
            self.select(unit);
        }

        // found a unit
        true
    }

    /// Attempts to select all units on screen
    pub fn do_select_all_units_on_screen_command(&mut self) {
        // get all units on the level
        let units = GameplayStatics::get_all_actors_of_class(
            self.as_ref().upcast_to(),
            SubclassOf::<StrategyUnit>::base().upcast_to(),
        );

        // process each unit
        for unit in units.into_iter().filter_map(|actor| actor.cast::<StrategyUnit>().ok()) {
            // is the unit is not already selected, and is on screen?
            let on_screen = unit
                .checked()
                .is_ok_and(|actor| actor.was_recently_rendered(Some(RECENTLY_RENDERED_TOLERANCE)));
            if !self.controlled_units().contains(&unit) && on_screen {
                // select the unit
                self.select(unit);
            }
        }
    }

    /// Deselects any selected units
    pub fn do_deselect_all_units_command(&mut self) {
        // deselect each unit
        for unit in self.controlled_units() {
            if let Ok(unit) = StrategyUnit::from_obj(unit) {
                unit.unit_deselected();
            }
        }

        // clear the selection list
        self.controlled_units_mut().clear();
    }

    /// Toggles between selecting all units on screen and deselecting units
    pub fn do_toggle_select_all_units_command(&mut self) {
        // do we have units selected?
        if !self.controlled_units().is_empty() {
            // deselect all units
            self.do_deselect_all_units_command();
        } else {
            // select all units on screen
            self.do_select_all_units_on_screen_command();
        }
    }

    /// Scrolls the camera based on a new screen coordinate
    pub fn do_camera_drag_scroll_command(&mut self, current_cursor_position: DVec2) {
        let offset = model::drag_scroll_offset(
            self.starting_drag_scroll_position(),
            current_cursor_position,
            self.drag_multiplier(),
        );

        // apply the offset to the camera pawn
        if let Ok(pawn) = self.controlled_camera_pawn().checked() {
            pawn.k2_add_actor_world_offset(&FVector::from_dvec3(offset), false, false);
        }
    }

    /// Attempts to move all selected units to the given location
    pub fn do_move_units_command(&mut self, goal_location: DVec3) {
        let units = self.controlled_units();
        if units.is_empty() {
            // no units selected, so just show negative cursor feedback
            self.bp_cursor_feedback(&FVector::from_dvec3(goal_location), false);
            return;
        }

        // find the closest unit to the goal
        let closest_unit = self.get_closest_selected_unit_to_location(goal_location);

        // tell each unit to move to the location
        for unit in &units {
            if let Ok(mut strategy_unit) = StrategyUnit::from_obj(*unit) {
                strategy_unit.move_to_location(goal_location, Some(*unit) == closest_unit, units.clone());
            }
        }

        // show positive cursor feedback
        self.bp_cursor_feedback(&FVector::from_dvec3(goal_location), true);
    }

    /// Applies a zoom change to the camera
    pub fn do_camera_modify_zoom_command(&mut self, zoom_delta: f32) {
        // add the delta, clamped between min and max
        let zoom = self.zoom_range().modified(self.camera_zoom(), zoom_delta);
        self.apply_zoom(zoom);
    }

    /// Resets the camera zoom to default
    pub fn do_camera_reset_zoom_command(&mut self) {
        self.apply_zoom(self.default_zoom());
    }

    /// Sets the camera zoom to a percentage between min and max zoom
    pub fn do_camera_set_zoom_percentage_command(&mut self, percentage: f32) {
        // lerp between min and max zoom
        let zoom = self.zoom_range().at_percentage(percentage);
        self.apply_zoom(zoom);
    }

    /// `SetupInputComponent` and the `BeginPlay` touch controls, on a local
    /// player controller.
    fn setup_input(&mut self) -> RustealResult<()> {
        let me: UObjectRef<PlayerController> = self.as_ref();
        // only spawn touch controls and set up input on local player controllers
        if !me.checked()?.is_local_player_controller() {
            return Ok(());
        }

        // add the input mapping context, chosen from the input mode
        let chosen_context = if self.should_use_touch_controls() {
            self.touch_mapping_context()
        } else {
            self.mouse_mapping_context()
        };
        enhanced_input_subsystem(me)?
            .checked()?
            .add_mapping_context(chosen_context, 0, &OwnedStruct::new());

        // Camera
        bind_action(&me, self.move_camera_action(), ETriggerEvent::Triggered, "MoveCamera")?;
        bind_action(&me, self.zoom_camera_action(), ETriggerEvent::Triggered, "ZoomCamera")?;
        bind_action(&me, self.reset_camera_action(), ETriggerEvent::Triggered, "ResetCamera")?;

        // Mouse Interaction
        let select_hold = self.select_hold_action();
        bind_action(&me, select_hold, ETriggerEvent::Started, "SelectHoldStarted")?;
        bind_action(&me, select_hold, ETriggerEvent::Triggered, "SelectHoldTriggered")?;
        bind_action(&me, select_hold, ETriggerEvent::Completed, "SelectHoldCompleted")?;
        bind_action(&me, select_hold, ETriggerEvent::Canceled, "SelectHoldCompleted")?;
        bind_action(&me, self.select_click_action(), ETriggerEvent::Completed, "SelectClick")?;
        bind_action(&me, self.select_click_additive_action(), ETriggerEvent::Completed, "SelectClickAdditive")?;
        bind_action(&me, self.select_all_double_click_action(), ETriggerEvent::Completed, "SelectAllDoubleClick")?;
        let interact_hold = self.interact_hold_action();
        bind_action(&me, interact_hold, ETriggerEvent::Started, "InteractHoldStarted")?;
        bind_action(&me, interact_hold, ETriggerEvent::Triggered, "InteractHoldTriggered")?;
        bind_action(&me, self.interact_click_action(), ETriggerEvent::Completed, "InteractClick")?;

        // Touch Interaction
        let touch_primary = self.touch_primary_hold_action();
        bind_action(&me, touch_primary, ETriggerEvent::Started, "TouchPrimaryHoldStarted")?;
        bind_action(&me, touch_primary, ETriggerEvent::Triggered, "TouchPrimaryHoldTriggered")?;
        bind_action(&me, touch_primary, ETriggerEvent::Completed, "TouchPrimaryHoldCompleted")?;
        let touch_secondary = self.touch_secondary_action();
        bind_action(&me, touch_secondary, ETriggerEvent::Triggered, "TouchSecondaryTriggered")?;
        bind_action(&me, touch_secondary, ETriggerEvent::Completed, "TouchSecondaryCompleted")?;
        bind_action(&me, touch_secondary, ETriggerEvent::Canceled, "TouchSecondaryCompleted")?;

        if self.should_use_touch_controls() {
            // spawn the mobile controls widget
            match create_widget_of_class(&me, self.mobile_controls_widget_class()) {
                Ok(widget) => {
                    // add the controls to the player screen
                    widget.checked()?.add_to_player_screen(Some(0));

                    // set the PC pointer on the mobile controls widget
                    let mut mobile_controls = StrategyTouchControls::from_obj(widget)?;
                    mobile_controls.set_player_controller(self.as_ref().cast()?);
                    self.set_mobile_controls_widget(widget);
                }
                Err(_) => ulog!(LOG_ERROR, "Could not spawn mobile controls widget."),
            }
        }
        Ok(())
    }

    /// Returns true if the PC should run using touchscreen controls
    fn should_use_touch_controls(&self) -> bool {
        // are we on a mobile platform? Should we force touch?
        should_display_touch_interface() || self.b_force_touch_controls()
    }

    /// Adds a unit to the selection list and notifies it
    fn select(&mut self, unit: UObjectRef<StrategyUnit>) {
        self.controlled_units_mut().push(unit);
        if let Ok(unit) = StrategyUnit::from_obj(unit) {
            unit.unit_selected();
        }
    }

    /// Updates the HUD's selection box, from its start to `selection_position`
    fn update_selection_box(&mut self, selection_position: DVec2) {
        // calculate the size of the selection box
        let start = self.starting_box_selection_position();
        let selection_size = selection_position - start;

        // update the selection box on the HUD
        if let Ok(mut hud) = StrategyHUD::from_obj(self.strategy_hud()) {
            hud.drag_select_update(start, selection_size, selection_position, true);
        }
    }

    /// Hides the HUD's selection box
    fn hide_selection_box(&mut self) {
        if let Ok(mut hud) = StrategyHUD::from_obj(self.strategy_hud()) {
            hud.drag_select_update(DVec2::ZERO, DVec2::ZERO, DVec2::ZERO, false);
        }
    }

    /// Sets the zoom on the camera pawn
    fn apply_zoom(&mut self, zoom: f32) {
        self.set_camera_zoom(zoom);
        if let Ok(pawn) = StrategyPawn::from_obj(self.controlled_camera_pawn()) {
            let _ = pawn.set_zoom_modifier(zoom);
        }
    }

    fn zoom_range(&self) -> ZoomRange {
        ZoomRange { min: self.min_zoom_level(), max: self.max_zoom_level(), default: self.default_zoom() }
    }

    fn game_time(&self) -> f64 {
        GameplayStatics::get_time_seconds(self.as_ref().upcast_to())
    }

    /// The selected unit closest to the target location
    fn get_closest_selected_unit_to_location(&self, target_location: DVec3) -> Option<UObjectRef<StrategyUnit>> {
        let (units, locations): (Vec<_>, Vec<_>) = self
            .controlled_units()
            .into_iter()
            .filter_map(|unit| Some((unit, unit.checked().ok()?.k2_get_actor_location().to_dvec3())))
            .unzip();
        model::closest_to(&locations, target_location).map(|index| units[index])
    }

    /// Calculates and returns the current mouse location
    fn get_mouse_location_for_player(&self) -> DVec2 {
        // attempt to get the mouse position from this PC
        match self.as_ref().checked().map(|me| me.get_mouse_position()) {
            Ok((true, mouse_x, mouse_y)) => DVec2::new(f64::from(mouse_x), f64::from(mouse_y)),
            // return an invalid vector
            _ => DVec2::ZERO,
        }
    }

    /// The world location under the cursor, if there is a blocking hit
    fn get_location_under_cursor(&self) -> Option<DVec3> {
        // trace the selection channel at the cursor location
        let me = self.as_ref().checked().ok()?;
        let (blocking_hit, out_hit) = me.get_hit_result_under_cursor_by_channel(self.selection_trace_channel(), false);

        // if there was a blocking hit, return the hit location
        blocking_hit.then(|| out_hit.as_ref().get_location().to_dvec3())
    }

    /// Projects the current touch location into world space
    fn project_touch_point_to_world_space(&self) -> DVec3 {
        let Ok(me) = self.as_ref().checked() else {
            return DVec3::ZERO;
        };

        // get the touch coordinates for the first finger
        let (touch_x, touch_y, _pressed) = me.get_input_touch_state(ETouchIndex::Touch1);

        // deproject the coords into world space
        let (deprojected, world_location, world_direction) = me.deproject_screen_position_to_world(touch_x, touch_y);
        if !deprojected {
            // failed to deproject, return a zero vector
            return DVec3::ZERO;
        }
        let (world_location, world_direction) = (world_location.to_dvec3(), world_direction.to_dvec3());

        // run a line trace down the camera on the visibility channel
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

        // if we hit something, return the impact point
        if hit {
            return out_hit.as_ref().get_impact_point().to_dvec3();
        }

        // intersect with a horizontal plane and return the resulting point
        model::line_plane_intersection(
            world_location,
            world_location + world_direction * TOUCH_PROJECTION_DISTANCE,
            DVec3::ZERO,
            DVec3::Z,
        )
    }
}
