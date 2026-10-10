use bindings::engine::{
    Actor, Controller, GameModeBase, GameplayStatics, KismetMathLibrary, PlayerStart,
};
use bindings::prelude::*;
use bindings::umg::UserWidgetExt;
use rusteal_runtime::runtime::{FName, SubclassOf, UObjectRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::model::{self, TeamScores};
use super::player_controller::ShooterPlayerController;
use super::ui::ShooterUI;

#[uclass(parent = GameModeBase)]
pub struct ShooterGameMode {
    #[uproperty(EditAnywhere, name = "ShooterUIClass", category = "Shooter")]
    shooter_ui_class: SubclassOf<ShooterUI>,

    #[uproperty(name = "ShooterUI")]
    shooter_ui: UObjectRef<ShooterUI>,

    #[uproperty(EditDefaultsOnly, category = "Local Multiplayer", default = 1)]
    number_of_local_players: i32,

    current_player_start_assignment: i32,

    team_scores: TeamScores,
}

#[uclass_impl]
impl ShooterGameMode {
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        let world = self.as_ref().upcast_to();

        let player_controller = GameplayStatics::get_player_controller(world, 0);

        if let Ok(ui) = create_widget_of_class(&player_controller, self.shooter_ui_class())
            && let Ok(widget) = ui.checked()
        {
            widget.add_to_viewport(Some(0));
            self.set_shooter_ui(ui);
        }

        for i in 2..=self.number_of_local_players() {
            let new_player = GameplayStatics::create_player(world, Some(-1), Some(true));

            if let Ok(mut new_player) = ShooterPlayerController::from_obj(new_player) {
                new_player.set_team(model::local_player_team(i));
            }
        }
    }

    #[ufunction(Override)]
    fn choose_player_start(&mut self, _player: UObjectRef<Controller>) -> UObjectRef<Actor> {
        let world = self.as_ref().upcast_to();
        let player_start_class = SubclassOf::<PlayerStart>::base().upcast_to::<Actor>();

        let player_tag = FName::new(&format!("Player{}", self.current_player_start_assignment()));

        let mut player_starts = GameplayStatics::get_all_actors_of_class_with_tag(
            world,
            player_start_class,
            player_tag.handle(),
        );

        self.set_current_player_start_assignment(self.current_player_start_assignment() + 1);

        if player_starts.is_empty() {
            player_starts = GameplayStatics::get_all_actors_of_class(world, player_start_class);
        }

        if player_starts.is_empty() {
            return UObjectRef::null();
        }

        let last = player_starts.len() as i32 - 1;

        player_starts[KismetMathLibrary::random_integer_in_range(0, last) as usize]
    }
}

impl ShooterGameMode {
    pub fn increment_team_score(&mut self, team_byte: u8) {
        let score = self.team_scores_mut().increment(team_byte);

        if let Ok(ui) = ShooterUI::from_obj(self.shooter_ui()) {
            ui.bp_update_score(team_byte, score);
        }
    }

    pub fn should_spawn_enemy_npcs(&self) -> bool {
        self.number_of_local_players() < 2
    }
}
