// HorrorUI: the Horror variant's `UHorrorUI` in Rust, the parent of the game
// UI widget `UI_Horror`, which shows the sprint meter. The player controller
// makes it its character's sprint listener (`setup_character`), and the
// character calls it as the C++ delegates call the C++ UI.

use bindings::umg::UserWidget;
use rusteal_runtime::runtime::RustealResult;
use rusteal_runtime::{uclass, uclass_impl};

use super::character::HorrorCharacter;

#[uclass(parent = UserWidget)]
pub struct HorrorUI {}

#[uclass_impl]
impl HorrorUI {
    /// Passes control to Blueprint to update the sprint meter widgets
    #[ufunction(BlueprintImplementableEvent, name = "BP_SprintMeterUpdated")]
    pub fn bp_sprint_meter_updated(&self, percent: f32) {}

    /// Passes control to Blueprint to update the sprint meter status
    #[ufunction(BlueprintImplementableEvent, name = "BP_SprintStateChanged")]
    pub fn bp_sprint_state_changed(&self, b_sprinting: bool) {}
}

impl HorrorUI {
    /// Sets up the sprint listener for the passed character
    pub fn setup_character(&self, horror_character: &HorrorCharacter) -> RustealResult<()> {
        horror_character.set_sprint_listener(self.as_ref().cast::<HorrorUI>()?);
        Ok(())
    }

    /// Called when the character's sprint meter is updated
    pub fn on_sprint_meter_updated(&self, percent: f32) {
        // call the BP handler
        self.bp_sprint_meter_updated(percent);
    }

    /// Called when the character's sprint state changes
    pub fn on_sprint_state_changed(&self, b_sprinting: bool) {
        // call the BP handler
        self.bp_sprint_state_changed(b_sprinting);
    }
}
