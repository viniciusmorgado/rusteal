// HorrorGameMode: the Horror variant's `AHorrorGameMode` in Rust. Spawns
// the extra local players and assigns each player its own
// PlayerStart (tagged `Player0`, `Player1`...). Its Blueprint child
// `BP_HorrorGameMode` sets the pawn and controller classes.

use bindings::engine::{
    Actor, Controller, GameModeBase, GameplayStatics, KismetMathLibrary, PlayerStart,
};
use rusteal_runtime::runtime::{FName, SubclassOf, UObjectRef};
use rusteal_runtime::{uclass, uclass_impl};

#[uclass(parent = GameModeBase)]
pub struct HorrorGameMode {
    /// Determines how many local players should be spawned on game start
    #[uproperty(EditDefaultsOnly, default = 1)]
    number_of_local_players: i32,

    /// Used to assign players to different PlayerStarts in the level
    current_player_start_assignment: i32,
}

#[uclass_impl]
impl HorrorGameMode {
    /// Initialization
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        // create each additional local player.
        // Player 0 will be created automatically as part of regular game init
        for _ in 2..=self.number_of_local_players() {
            GameplayStatics::create_player(self.as_ref().upcast_to(), Some(-1), Some(true));
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
