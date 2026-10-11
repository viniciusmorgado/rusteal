use bindings::umg::UserWidget;
use rusteal_runtime::{uclass, uclass_impl};

use crate::model;

#[uclass(parent = UserWidget)]
pub struct VehicleAdvUI {
    #[uproperty(EditAnywhere, BlueprintReadOnly, name = "bIsMPH", category = "Vehicle", default = false)]
    b_is_mph: bool,
}

#[uclass_impl]
impl VehicleAdvUI {
    #[ufunction(BlueprintImplementableEvent)]
    fn on_speed_update(&self, new_speed: f32) {}

    #[ufunction(BlueprintImplementableEvent)]
    fn on_gear_update(&self, new_gear: i32) {}
}

impl VehicleAdvUI {
    pub fn update_speed(&self, new_speed: f32) {
        self.on_speed_update(model::display_speed(new_speed, self.b_is_mph()));
    }

    pub fn update_gear(&self, new_gear: i32) {
        self.on_gear_update(new_gear);
    }
}
