use bindings::umg::UserWidget;
use rusteal_runtime::uclass;
use rusteal_runtime::uclass_impl;

#[uclass(parent = UserWidget)]
pub struct SideScrollingUI {}

#[uclass_impl]
impl SideScrollingUI {
    #[ufunction(BlueprintImplementableEvent)]
    pub fn update_pickups(&self, amount: i32) {}
}
