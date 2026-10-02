// CombatLifeBar: the Combat variant's `UCombatLifeBar` in Rust, the parent of
// the life bar widget `UI_LifeBar` the characters show above their heads.

use bindings::umg::UserWidget;
use rusteal_runtime::runtime::OwnedStruct;
use rusteal_runtime::{uclass, uclass_impl};

use bindings::core_ue::FLinearColor;

#[uclass(parent = UserWidget)]
pub struct CombatLifeBar {}

#[uclass_impl]
impl CombatLifeBar {
    /// Sets the life bar to the provided 0-1 percentage value
    #[ufunction(BlueprintImplementableEvent)]
    pub fn set_life_percentage(&self, percent: f32) {}

    /// Sets the life bar fill color
    #[ufunction(BlueprintImplementableEvent)]
    pub fn set_bar_color(&self, color: &OwnedStruct<FLinearColor>) {}
}
