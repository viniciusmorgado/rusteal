// StrategyHUD: the Strategy variant's `AStrategyHUD` in Rust. Spawns the game
// UI, draws the box selection the player drags (and selects the units in
// it) and marks each selected unit.
//
// `DrawHUD` is a C++ virtual; here it is the `ReceiveDrawHUD` event, which
// the engine raises while the HUD draws.

use bindings::engine::{ActorExt, HUD, HUDExt, PlayerControllerExt};
use bindings::prelude::*;
use bindings::umg::UserWidgetExt;
use glam::DVec2;
use rusteal_runtime::runtime::{
    LOG_ERROR, LinearColor, OwnedStruct, RustealResult, SubclassOf, UObjectRef, ulog,
};
use rusteal_runtime::{uclass, uclass_impl};

use super::player_controller::StrategyPlayerController;
use super::ui::StrategyUI;
use super::unit::StrategyUnit;

/// The label drawn near each selected unit, its offset and scale.
const SELECTION_STRING: &str = "Selected";
const SELECTION_TEXT_OFFSET: DVec2 = DVec2::new(-25.0, 25.0);
const SELECTION_TEXT_SCALE: f32 = 1.5;

/// Simple strategy game HUD. Draws the selection box and unit selected overlays
#[uclass(parent = HUD)]
pub struct StrategyHUD {
    /// Pointer to the UI user widget
    #[uproperty]
    ui_widget: UObjectRef<StrategyUI>,

    /// Type of UI Widget to spawn
    #[uproperty(EditAnywhere, name = "UIWidgetClass", category = "UI")]
    ui_widget_class: SubclassOf<StrategyUI>,

    /// Color of the selection box
    #[uproperty(EditAnywhere, category = "UI")]
    selection_box_color: OwnedStruct<FLinearColor>,

    /// If true, the HUD will draw the selection box
    b_draw_box: bool,

    /// Starting coords of the selection box
    box_start: DVec2,

    /// Width and height of the selection box
    box_size: DVec2,

    /// Current position of the selection box
    box_current_position: DVec2,
}

#[uclass_impl]
impl StrategyHUD {
    /// Initialization
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let Err(e) = self.spawn_ui() {
            ulog!(LOG_ERROR, "[Strategy] could not spawn the UI widget: {e}");
        }
    }

    /// Draws the HUD
    #[ufunction(Override)]
    fn receive_draw_hud(&mut self, _size_x: i32, _size_y: i32) {
        // ensure we have a valid player controller
        let Ok(me) = self.as_ref().checked() else {
            return;
        };
        let owning_player = me.get_owning_player_controller();
        let Ok(mut player_controller) = StrategyPlayerController::from_obj(owning_player) else {
            return;
        };

        // draw the selection box
        if self.b_draw_box() {
            let (start, size) = (self.box_start(), self.box_size());
            me.draw_rect(&self.selection_box_color(), start.x as f32, start.y as f32, size.x as f32, size.y as f32);

            // get all the units in the selection box
            let boxed_units: Vec<UObjectRef<StrategyUnit>> = me
                .get_actors_in_selection_rectangle(
                    SubclassOf::<StrategyUnit>::base().upcast_to(),
                    &FVector2D::from_dvec2(start),
                    &FVector2D::from_dvec2(self.box_current_position()),
                    Some(true),
                    Some(false),
                )
                .into_iter()
                .filter_map(|actor| actor.cast().ok())
                .collect();

            // update the unit selection on the player controller
            player_controller.drag_select_units(boxed_units);
        }

        // get the currently selected units
        let selected_units = player_controller.get_selected_units();

        // update the selection count on the UI widget
        if let Ok(mut ui_widget) = StrategyUI::from_obj(self.ui_widget()) {
            ui_widget.set_selected_units_count(selected_units.len() as i32);
        }

        // process each selected unit
        let Ok(owning_player) = owning_player.checked() else {
            return;
        };
        let white = FLinearColor::from_linear_color(LinearColor::new(1.0, 1.0, 1.0, 1.0));
        for unit in selected_units {
            let Ok(unit) = unit.checked() else {
                continue;
            };

            // project the unit's location to screen coordinates
            let (projected, screen_coords) =
                owning_player.project_world_location_to_screen(&unit.k2_get_actor_location(), Some(true));
            if projected {
                // draw a selection string near the unit
                let text_position = screen_coords.to_dvec2() + SELECTION_TEXT_OFFSET;
                me.draw_text(
                    SELECTION_STRING,
                    &white,
                    text_position.x as f32,
                    text_position.y as f32,
                    None,
                    Some(SELECTION_TEXT_SCALE),
                    Some(false),
                );
            }
        }
    }
}

impl StrategyHUD {
    /// Updates the drag selection box
    pub fn drag_select_update(&mut self, start: DVec2, width_and_height: DVec2, current_position: DVec2, b_draw: bool) {
        // copy the selection box data
        self.set_b_draw_box(b_draw);
        self.set_box_start(start);
        self.set_box_size(width_and_height);
        self.set_box_current_position(current_position);
    }

    /// `BeginPlay`: the UI widget, on the owning player's screen.
    fn spawn_ui(&mut self) -> RustealResult<()> {
        let owning_player = self.as_ref().checked()?.get_owning_player_controller();
        let ui_widget = create_widget_of_class(&owning_player, self.ui_widget_class())?;

        // add the UI widget to the screen
        ui_widget.checked()?.add_to_viewport(Some(0));
        self.set_ui_widget(ui_widget);
        Ok(())
    }
}

