// ShooterGameMode: the Shooter variant's `AShooterGameMode` in Rust. Shows
// the scoreboard, keeps the team scores, spawns the extra local players
// (alternating teams) and assigns each player its own PlayerStart. Its
// Blueprint child `BP_ShooterGameMode` sets the pawn, controller and UI
// classes.

use bindings::engine::{Actor, Controller, GameModeBase, GameplayStatics, KismetMathLibrary, PlayerStart};
use bindings::prelude::*;
use bindings::umg::UserWidgetExt;
use rusteal_runtime::runtime::{FName, SubclassOf, UObjectRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::model::{self, TeamScores};
use super::player_controller::ShooterPlayerController;
use super::ui::ShooterUI;

#[uclass(parent = GameModeBase)]
pub struct ShooterGameMode {
    /// Type of UI widget to spawn
    #[uproperty(EditAnywhere, name = "ShooterUIClass", category = "Shooter")]
    shooter_ui_class: SubclassOf<ShooterUI>,

    /// Pointer to the UI widget
    #[uproperty(name = "ShooterUI")]
    shooter_ui: UObjectRef<ShooterUI>,

    /// Determines how many local players should be spawned on game start
    #[uproperty(EditDefaultsOnly, category = "Local Multiplayer", default = 1)]
    number_of_local_players: i32,

    /// Used to assign players to different PlayerStarts in the level
    current_player_start_assignment: i32,

    /// Map of scores by team ID
    team_scores: TeamScores,
}

#[uclass_impl]
impl ShooterGameMode {
    /// Gameplay initialization
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        let world = self.as_ref().upcast_to();

        // create the UI
        let player_controller = GameplayStatics::get_player_controller(world, 0);
        if let Ok(ui) = create_widget_of_class(&player_controller, self.shooter_ui_class())
            && let Ok(widget) = ui.checked()
        {
            widget.add_to_viewport(Some(0));
            self.set_shooter_ui(ui);
        }

        // create each additional local player.
        // Player 0 will be created automatically as part of regular game init
        for i in 2..=self.number_of_local_players() {
            let new_player = GameplayStatics::create_player(world, Some(-1), Some(true));
            if let Ok(mut new_player) = ShooterPlayerController::from_obj(new_player) {
                new_player.set_team(model::local_player_team(i));
            }
        }
    }

    /// Assigns a PlayerStart to a specific player
    #[ufunction(Override)]
    fn choose_player_start(&mut self, _player: UObjectRef<Controller>) -> UObjectRef<Actor> {
        let world = self.as_ref().upcast_to();
        let player_start_class = SubclassOf::<PlayerStart>::base().upcast_to::<Actor>();

        // build the current player tag
        let player_tag = FName::new(&format!("Player{}", self.current_player_start_assignment()));

        // find all player starts with the matching player tag
        let mut player_starts =
            GameplayStatics::get_all_actors_of_class_with_tag(world, player_start_class, player_tag.handle());

        // increment the player start assignment index
        self.set_current_player_start_assignment(self.current_player_start_assignment() + 1);

        // if no PlayerStarts were found, default to all PlayerStarts instead
        if player_starts.is_empty() {
            player_starts = GameplayStatics::get_all_actors_of_class(world, player_start_class);
        }

        // have we found at least one PlayerStart?
        if player_starts.is_empty() {
            // no PlayerStarts in the level
            return UObjectRef::null();
        }
        let last = player_starts.len() as i32 - 1;
        player_starts[KismetMathLibrary::random_integer_in_range(0, last) as usize]
    }
}

impl ShooterGameMode {
    /// Increases the score for the given team
    pub fn increment_team_score(&mut self, team_byte: u8) {
        // increment the score for the given team
        let score = self.team_scores_mut().increment(team_byte);

        // update the UI
        if let Ok(ui) = ShooterUI::from_obj(self.shooter_ui()) {
            ui.bp_update_score(team_byte, score);
        }
    }

    /// Returns true if enemy NPCs should be used
    pub fn should_spawn_enemy_npcs(&self) -> bool {
        // only spawn enemy NPCs in single player mode
        self.number_of_local_players() < 2
    }
}
