// SideScrollingUI: the SideScrolling variant's `USideScrollingUI` in Rust, the
// parent of the game UI widget `UI_SideScrolling`, which shows the pickups
// counter.

use bindings::umg::UserWidget;
use rusteal_runtime::uclass;
use rusteal_runtime::uclass_impl;

#[uclass(parent = UserWidget)]
pub struct SideScrollingUI {}

#[uclass_impl]
impl SideScrollingUI {
    /// Update the widget with the number of pickups collected
    #[ufunction(BlueprintImplementableEvent)]
    pub fn update_pickups(&self, amount: i32) {}
}
