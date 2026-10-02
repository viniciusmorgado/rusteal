// StrategyUI: the Strategy variant's `UStrategyUI` in Rust, the parent of the
// game UI widget `UI_Strategy`, which shows how many units are selected.

use bindings::umg::UserWidget;
use rusteal_runtime::{uclass, uclass_impl};

/// Simple UI widget for the strategy game. Keeps track of the number of
/// units currently selected
#[uclass(parent = UserWidget)]
pub struct StrategyUI {
    /// Number of units currently selected
    selected_unit_count: i32,
}

#[uclass_impl]
impl StrategyUI {
    /// Blueprint handler to update unit count sub-widgets
    #[ufunction(BlueprintImplementableEvent, name = "BP_UpdateUnitsCount")]
    fn bp_update_units_count(&self) {}

    /// Returns the number of units selected
    #[ufunction(BlueprintPure)]
    fn get_selected_units_count(&self) -> i32 {
        self.selected_unit_count()
    }
}

impl StrategyUI {
    /// Sets the number of units selected
    pub fn set_selected_units_count(&mut self, count: i32) {
        // is this a different count?
        let changed = self.selected_unit_count() != count;

        // update the counter
        self.set_selected_unit_count(count);

        // if the count changed, call the BP handler
        if changed {
            self.bp_update_units_count();
        }
    }
}
