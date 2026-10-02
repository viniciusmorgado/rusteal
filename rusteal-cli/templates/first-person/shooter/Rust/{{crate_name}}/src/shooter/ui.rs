// The Shooter variant's UI widget parents in Rust: `UShooterUI`, the
// scoreboard (`UI_Shooter`), and `UShooterBulletCounterUI`, the player's
// bullet counter and life bar (`UI_ShooterBulletCounter`). Their Blueprint
// children draw them.

use bindings::umg::UserWidget;
use rusteal_runtime::{uclass, uclass_impl};

/// Simple scoreboard UI for a first person shooter game
#[uclass(parent = UserWidget)]
pub struct ShooterUI {}

#[uclass_impl]
impl ShooterUI {
    /// Allows Blueprint to update score sub-widgets
    #[ufunction(BlueprintImplementableEvent, name = "BP_UpdateScore")]
    pub fn bp_update_score(&self, team_byte: u8, score: i32) {}
}

/// Simple bullet counter UI widget for a first person shooter game
#[uclass(parent = UserWidget)]
pub struct ShooterBulletCounterUI {}

#[uclass_impl]
impl ShooterBulletCounterUI {
    /// Allows Blueprint to update sub-widgets with the new bullet count
    #[ufunction(BlueprintImplementableEvent, name = "BP_UpdateBulletCounter")]
    pub fn bp_update_bullet_counter(&self, magazine_size: i32, bullet_count: i32) {}

    /// Allows Blueprint to update sub-widgets with the new life total and play a damage effect on the HUD
    #[ufunction(BlueprintImplementableEvent, name = "BP_Damaged")]
    pub fn bp_damaged(&self, life_percent: f32) {}
}
