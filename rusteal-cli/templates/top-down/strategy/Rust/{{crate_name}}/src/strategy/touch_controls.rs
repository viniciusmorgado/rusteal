use bindings::umg::UserWidget;
use rusteal_runtime::runtime::UObjectRef;
use rusteal_runtime::{uclass, uclass_impl};

use super::player_controller::StrategyPlayerController;

#[uclass(parent = UserWidget)]
pub struct StrategyTouchControls {
    player_controller: UObjectRef<StrategyPlayerController>,
}

#[uclass_impl]
impl StrategyTouchControls {
    #[ufunction(BlueprintImplementableEvent, name = "BP_SetZoomPercentage")]
    pub fn bp_set_zoom_percentage(&self, percentage: f32) {}

    #[ufunction(BlueprintCallable)]
    fn reset_zoom(&mut self) {
        if let Ok(mut player_controller) =
            StrategyPlayerController::from_obj(self.player_controller())
        {
            player_controller.do_camera_reset_zoom_command();
            self.bp_set_zoom_percentage(player_controller.get_default_zoom_percentage());
        }
    }

    #[ufunction(BlueprintCallable)]
    fn toggle_select_all_units(&mut self) {
        if let Ok(mut player_controller) =
            StrategyPlayerController::from_obj(self.player_controller())
        {
            player_controller.do_toggle_select_all_units_command();
        }
    }

    #[ufunction(BlueprintCallable)]
    fn set_zoom_percentage(&mut self, percentage: f32) {
        if let Ok(mut player_controller) =
            StrategyPlayerController::from_obj(self.player_controller())
        {
            player_controller.do_camera_set_zoom_percentage_command(percentage);
        }
    }
}
