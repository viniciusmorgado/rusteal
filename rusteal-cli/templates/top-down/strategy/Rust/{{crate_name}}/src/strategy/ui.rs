use bindings::umg::UserWidget;
use rusteal_runtime::{uclass, uclass_impl};

#[uclass(parent = UserWidget)]
pub struct StrategyUI {
    selected_unit_count: i32,
}

#[uclass_impl]
impl StrategyUI {
    #[ufunction(BlueprintImplementableEvent, name = "BP_UpdateUnitsCount")]
    fn bp_update_units_count(&self) {}

    #[ufunction(BlueprintPure)]
    fn get_selected_units_count(&self) -> i32 {
        self.selected_unit_count()
    }
}

impl StrategyUI {
    pub fn set_selected_units_count(&mut self, count: i32) {
        let changed = self.selected_unit_count() != count;

        self.set_selected_unit_count(count);

        if changed {
            self.bp_update_units_count();
        }
    }
}
