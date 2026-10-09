use bindings::umg::UserWidget;
use rusteal_runtime::runtime::OwnedStruct;
use rusteal_runtime::{uclass, uclass_impl};

use bindings::core_ue::FLinearColor;

#[uclass(parent = UserWidget)]
pub struct CombatLifeBar {}

#[uclass_impl]
impl CombatLifeBar {
    #[ufunction(BlueprintImplementableEvent)]
    pub fn set_life_percentage(&self, percent: f32) {}

    #[ufunction(BlueprintImplementableEvent)]
    pub fn set_bar_color(&self, color: &OwnedStruct<FLinearColor>) {}
}
