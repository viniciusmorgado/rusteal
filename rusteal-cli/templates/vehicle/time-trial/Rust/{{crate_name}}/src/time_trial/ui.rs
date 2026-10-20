use bindings::prelude::*;
use bindings::umg::{UserWidget, UserWidgetExt, WidgetExt};
use rusteal_runtime::runtime::{RustealResult, SubclassOf, UObjectRef, method};
use rusteal_runtime::{uclass, uclass_impl};

use super::model::LapTimes;

#[uclass(parent = UserWidget)]
pub struct TimeTrialUI {
    #[uproperty(EditAnywhere, name = "StartUIClass", category = "Start Countdown")]
    start_ui_class: SubclassOf<TimeTrialStartUI>,

    #[uproperty]
    start_ui: UObjectRef<TimeTrialStartUI>,

    laps: LapTimes,
}

#[uclass_impl]
impl TimeTrialUI {
    #[udelegate]
    pub fn on_race_start(&self) {}

    #[ufunction(Override)]
    fn construct(&mut self) {
        if let Err(e) = self.start_countdown() {
            log::error!("[TimeTrial] could not start the countdown: {e}");
        }
    }

    #[ufunction(BlueprintImplementableEvent, name = "BP_UpdateLaps")]
    fn bp_update_laps(&self) {}

    #[ufunction]
    fn start_race(&mut self) {
        self.on_race_start();
    }

    #[ufunction(BlueprintPure)]
    fn get_current_lap(&self) -> i32 {
        self.laps().current_lap
    }

    #[ufunction(BlueprintPure)]
    fn get_best_lap_time(&self) -> f32 {
        self.laps().best_lap_time
    }

    #[ufunction(BlueprintPure)]
    fn get_lap_start_time(&self) -> f32 {
        self.laps().lap_start_time
    }
}

impl TimeTrialUI {
    pub fn update_lap_count(&mut self, lap: i32, new_lap_start_time: f32) {
        self.laps_mut().start_lap(lap, new_lap_start_time);
        self.bp_update_laps();
    }

    fn start_countdown(&mut self) -> RustealResult<()> {
        let owning_player = self.as_ref().checked()?.get_owning_player();
        let start_ui = create_widget_of_class(&owning_player, self.start_ui_class())?;

        start_ui
            .upcast_to::<UserWidget>()
            .checked()?
            .add_to_viewport(Some(0));
        self.set_start_ui(start_ui);

        let countdown = TimeTrialStartUI::from_obj(start_ui)?;
        countdown
            .on_countdown_finished_add(method(self, Self::start_race))?
            .detach();
        countdown.start_countdown();

        Ok(())
    }
}

#[uclass(parent = UserWidget)]
pub struct TimeTrialStartUI {}

#[uclass_impl]
impl TimeTrialStartUI {
    #[udelegate]
    pub fn on_countdown_finished(&self) {}

    #[ufunction(BlueprintImplementableEvent, name = "BP_StartCountdown")]
    fn bp_start_countdown(&self) {}

    #[ufunction(BlueprintCallable)]
    fn finish_countdown(&mut self) {
        self.on_countdown_finished();
    }
}

impl TimeTrialStartUI {
    pub fn start_countdown(&self) {
        self.bp_start_countdown();
    }
}
