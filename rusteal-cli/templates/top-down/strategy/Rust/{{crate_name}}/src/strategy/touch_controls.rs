// StrategyTouchControls: the Strategy variant's `UStrategyTouchControls` in
// Rust, the parent of the touchscreen controls widget
// `WBP_StrategyMobileControls`, which exposes some game commands to its
// buttons and zoom slider.

use bindings::umg::UserWidget;
use rusteal_runtime::runtime::UObjectRef;
use rusteal_runtime::{uclass, uclass_impl};

use super::player_controller::StrategyPlayerController;

/// Base class for additional touchscreen controls for a strategy game.
/// Exposes some game commands to UI
#[uclass(parent = UserWidget)]
pub struct StrategyTouchControls {
    /// Pointer to the owning Strategy PC
    player_controller: UObjectRef<StrategyPlayerController>,
}

#[uclass_impl]
impl StrategyTouchControls {
    /// Syncs the camera zoom percentage with the UI. Called by the owning PC
    #[ufunction(BlueprintImplementableEvent, name = "BP_SetZoomPercentage")]
    pub fn bp_set_zoom_percentage(&self, percentage: f32) {}

    /// Resets the camera zoom level
    #[ufunction(BlueprintCallable)]
    fn reset_zoom(&mut self) {
        if let Ok(mut player_controller) = StrategyPlayerController::from_obj(self.player_controller()) {
            player_controller.do_camera_reset_zoom_command();
            self.bp_set_zoom_percentage(player_controller.get_default_zoom_percentage());
        }
    }

    /// Toggles between select all units and deselect all units.
    #[ufunction(BlueprintCallable)]
    fn toggle_select_all_units(&mut self) {
        if let Ok(mut player_controller) = StrategyPlayerController::from_obj(self.player_controller()) {
            player_controller.do_toggle_select_all_units_command();
        }
    }

    /// Sets the camera zoom percentage level
    #[ufunction(BlueprintCallable)]
    fn set_zoom_percentage(&mut self, percentage: f32) {
        if let Ok(mut player_controller) = StrategyPlayerController::from_obj(self.player_controller()) {
            player_controller.do_camera_set_zoom_percentage_command(percentage);
        }
    }
}
