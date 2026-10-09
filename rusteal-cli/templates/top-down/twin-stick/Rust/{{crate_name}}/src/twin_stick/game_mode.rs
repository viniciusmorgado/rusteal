use bindings::engine::{GameModeBase, GameplayStatics, KismetSystemLibrary};
use bindings::prelude::*;
use bindings::umg::UserWidgetExt;
use rusteal_runtime::runtime::{SubclassOf, UObjectRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::model::{Combo, ComboUpdate};
use super::ui::TwinStickUI;

#[uclass(parent = GameModeBase)]
pub struct TwinStickGameMode {
    #[uproperty(EditAnywhere, category = "Twin Stick")]
    ui_widget_class: SubclassOf<TwinStickUI>,

    #[uproperty]
    ui_widget: UObjectRef<TwinStickUI>,

    #[uproperty(EditAnywhere, category = "Twin Stick", default = 5)]
    combo_increment_max: i32,

    #[uproperty(EditAnywhere, category = "Twin Stick", default = 4)]
    combo_cap: i32,

    #[uproperty(EditAnywhere, category = "Twin Stick", default = 3.0)]
    combo_cooldown: f32,

    #[uproperty(EditAnywhere, name = "NPCCap", category = "Twin Stick", default = 20)]
    npc_cap: i32,

    combo: Combo,

    npc_count: i32,
}

#[uclass_impl]
impl TwinStickGameMode {
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        self.create_ui();
    }

    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), "ResetCombo");
    }

    #[ufunction]
    fn reset_combo(&mut self) {
        let mut combo = self.combo();
        let update = combo.cool_down();
        self.set_combo(combo);
        self.apply(update);
    }
}

impl TwinStickGameMode {
    pub fn item_used(&mut self, value: i32) {
        self.create_ui();

        if let Ok(ui) = TwinStickUI::from_obj(self.ui_widget()) {
            ui.update_items(value);
        }
    }

    pub fn score_update(&mut self, value: i32) {
        let mut combo = self.combo();
        let update = combo.add_score(value, self.combo_increment_max(), self.combo_cap());
        self.set_combo(combo);

        if let Ok(ui) = TwinStickUI::from_obj(self.ui_widget()) {
            ui.update_score(combo.score);
        }

        self.apply(update);
    }

    pub fn can_spawn_npcs(&self) -> bool {
        self.npc_count() < self.npc_cap()
    }

    pub fn increase_npcs(&mut self) {
        self.set_npc_count(self.npc_count() + 1);
    }

    pub fn decrease_npcs(&mut self) {
        self.set_npc_count(self.npc_count() - 1);
    }

    fn create_ui(&mut self) {
        if self.ui_widget().is_valid() {
            return;
        }

        let player_controller =
            GameplayStatics::get_player_controller(self.as_ref().upcast_to(), 0);

        if let Ok(widget) = create_widget_of_class(&player_controller, self.ui_widget_class())
            && let Ok(ui) = widget.checked()
        {
            ui.add_to_viewport(Some(0));
            self.set_ui_widget(widget);
        }
    }

    fn apply(&self, update: ComboUpdate) {
        if update.combo_changed
            && let Ok(ui) = TwinStickUI::from_obj(self.ui_widget())
        {
            ui.update_combo(self.combo().combo);
        }

        if update.restart_cooldown {
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
