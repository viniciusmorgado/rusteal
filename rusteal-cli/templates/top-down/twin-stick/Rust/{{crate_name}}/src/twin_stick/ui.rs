// TwinStickUI: the TwinStick variant's `UTwinStickUI` in Rust, the parent of
// the game UI widget `UI_TwinStick`, which shows the score, the combo
// multiplier and the items.

use bindings::umg::UserWidget;
use rusteal_runtime::{uclass, uclass_impl};

#[uclass(parent = UserWidget)]
pub struct TwinStickUI {}

#[uclass_impl]
impl TwinStickUI {
    /// Blueprint handler to update the items counter
    #[ufunction(BlueprintImplementableEvent)]
    pub fn update_items(&self, score: i32) {}

    /// Blueprint handler to update the score sub-widgets
    #[ufunction(BlueprintImplementableEvent)]
    pub fn update_score(&self, score: i32) {}

    /// Blueprint handler to update the combo sub-widgets
    #[ufunction(BlueprintImplementableEvent)]
    pub fn update_combo(&self, combo: i32) {}
}
