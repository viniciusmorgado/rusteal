// TwinStickGameMode: the TwinStick variant's `ATwinStickGameMode` in Rust.
// Keeps the score and its combo multiplier, shows them on the UI, and caps
// the number of NPCs in the level. Its Blueprint child `BP_TwinStickGameMode`
// sets the pawn, controller and UI classes.

use bindings::engine::{GameModeBase, GameplayStatics, KismetSystemLibrary};
use bindings::prelude::*;
use bindings::umg::UserWidgetExt;
use rusteal_runtime::runtime::{SubclassOf, UObjectRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::model::{Combo, ComboUpdate};
use super::ui::TwinStickUI;

#[uclass(parent = GameModeBase)]
pub struct TwinStickGameMode {
    /// Type of UI Widget to spawn
    #[uproperty(EditAnywhere, category = "Twin Stick")]
    ui_widget_class: SubclassOf<TwinStickUI>,

    /// Pointer to the spawned UI Widget
    #[uproperty]
    ui_widget: UObjectRef<TwinStickUI>,

    /// Number of combo hits to process before incrementing the combo multiplier
    #[uproperty(EditAnywhere, category = "Twin Stick", default = 5)]
    combo_increment_max: i32,

    /// Maximum allowed combo multiplier value
    #[uproperty(EditAnywhere, category = "Twin Stick", default = 4)]
    combo_cap: i32,

    /// Max time between kills before the combo multiplier resets
    #[uproperty(EditAnywhere, category = "Twin Stick", default = 3.0)]
    combo_cooldown: f32,

    /// Max number of NPCs to allow in the level at once
    #[uproperty(EditAnywhere, name = "NPCCap", category = "Twin Stick", default = 20)]
    npc_cap: i32,

    /// The score and its combo multiplier
    combo: Combo,

    /// Current number of NPCs in the level
    npc_count: i32,
}

#[uclass_impl]
impl TwinStickGameMode {
    /// Gameplay initialization
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        // create the UI widget if it hasn't already
        self.create_ui();
    }

    /// Cleanup
    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        // clear the combo timer
        KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), "ResetCombo");
    }

    /// Resets the combo multiplier after the cooldown time expires
    #[ufunction]
    fn reset_combo(&mut self) {
        let mut combo = self.combo();
        let update = combo.cool_down();
        self.set_combo(combo);
        self.apply(update);
    }
}

impl TwinStickGameMode {
    /// Called when an item has been used
    pub fn item_used(&mut self, value: i32) {
        // ensure the UI widget is available
        self.create_ui();

        // update the UI
        if let Ok(ui) = TwinStickUI::from_obj(self.ui_widget()) {
            ui.update_items(value);
        }
    }

    /// Increments the score by the given value
    pub fn score_update(&mut self, value: i32) {
        let mut combo = self.combo();
        let update = combo.add_score(value, self.combo_increment_max(), self.combo_cap());
        self.set_combo(combo);

        // update the UI
        if let Ok(ui) = TwinStickUI::from_obj(self.ui_widget()) {
            ui.update_score(combo.score);
        }

        // update the combo multiplier
        self.apply(update);
    }

    /// Returns true if the number of NPCs is under the cap
    pub fn can_spawn_npcs(&self) -> bool {
        self.npc_count() < self.npc_cap()
    }

    /// Increases the NPC count
    pub fn increase_npcs(&mut self) {
        self.set_npc_count(self.npc_count() + 1);
    }

    /// Decreases the NPC count
    pub fn decrease_npcs(&mut self) {
        self.set_npc_count(self.npc_count() - 1);
    }

    /// Creates the UI widget if it hasn't been created already
    fn create_ui(&mut self) {
        // avoid creating the UI multiple times
        if self.ui_widget().is_valid() {
            return;
        }

        // create the UI widget and add it to the viewport
        let player_controller = GameplayStatics::get_player_controller(self.as_ref().upcast_to(), 0);
        if let Ok(widget) = create_widget_of_class(&player_controller, self.ui_widget_class())
            && let Ok(ui) = widget.checked()
        {
            ui.add_to_viewport(Some(0));
            self.set_ui_widget(widget);
        }
    }

    /// Shows a changed combo multiplier and restarts its cooldown timer.
    fn apply(&self, update: ComboUpdate) {
        if update.combo_changed
            && let Ok(ui) = TwinStickUI::from_obj(self.ui_widget())
        {
            ui.update_combo(self.combo().combo);
        }
        if update.restart_cooldown {
            // reset the combo cooldown timer
            KismetSystemLibrary::k2_set_timer(
                self.as_ref().upcast_to(),
                "ResetCombo",
                self.combo_cooldown(),
                false,
                None,
                None,
                None,
            );
        }
    }
}
