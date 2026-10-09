use bindings::umg::UserWidget;
use rusteal_runtime::{uclass, uclass_impl};

#[uclass(parent = UserWidget)]
pub struct ShooterUI {}

#[uclass_impl]
impl ShooterUI {
    #[ufunction(BlueprintImplementableEvent, name = "BP_UpdateScore")]
    pub fn bp_update_score(&self, team_byte: u8, score: i32) {}
}

#[uclass(parent = UserWidget)]
pub struct ShooterBulletCounterUI {}

#[uclass_impl]
impl ShooterBulletCounterUI {
    #[ufunction(BlueprintImplementableEvent, name = "BP_UpdateBulletCounter")]
    pub fn bp_update_bullet_counter(&self, magazine_size: i32, bullet_count: i32) {}

    #[ufunction(BlueprintImplementableEvent, name = "BP_Damaged")]
    pub fn bp_damaged(&self, life_percent: f32) {}
}
