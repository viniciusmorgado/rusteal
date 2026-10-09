use bindings::umg::UserWidget;
use rusteal_runtime::{uclass, uclass_impl};

#[uclass(parent = UserWidget)]
pub struct TwinStickUI {}

#[uclass_impl]
impl TwinStickUI {
    #[ufunction(BlueprintImplementableEvent)]
    pub fn update_items(&self, score: i32) {}

    #[ufunction(BlueprintImplementableEvent)]
    pub fn update_score(&self, score: i32) {}

    #[ufunction(BlueprintImplementableEvent)]
    pub fn update_combo(&self, combo: i32) {}
}
