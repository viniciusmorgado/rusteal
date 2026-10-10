use bindings::umg::UserWidget;
use rusteal_runtime::runtime::RustealResult;
use rusteal_runtime::{uclass, uclass_impl};

use super::character::HorrorCharacter;

#[uclass(parent = UserWidget)]
pub struct HorrorUI {}

#[uclass_impl]
impl HorrorUI {
    #[ufunction(BlueprintImplementableEvent, name = "BP_SprintMeterUpdated")]
    pub fn bp_sprint_meter_updated(&self, percent: f32) {}

    #[ufunction(BlueprintImplementableEvent, name = "BP_SprintStateChanged")]
    pub fn bp_sprint_state_changed(&self, b_sprinting: bool) {}
}

impl HorrorUI {
    pub fn setup_character(&self, horror_character: &HorrorCharacter) -> RustealResult<()> {
        horror_character.set_sprint_listener(self.as_ref().cast::<HorrorUI>()?);

        Ok(())
    }

    pub fn on_sprint_meter_updated(&self, percent: f32) {
        self.bp_sprint_meter_updated(percent);
    }

    pub fn on_sprint_state_changed(&self, b_sprinting: bool) {
        self.bp_sprint_state_changed(b_sprinting);
    }
}
