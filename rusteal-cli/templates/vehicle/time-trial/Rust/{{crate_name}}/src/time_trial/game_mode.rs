use bindings::engine::{
    Actor, Controller, GameModeBase, GameplayStatics, KismetMathLibrary, PlayerStart,
};
use rusteal_runtime::runtime::{FName, SubclassOf, UObjectRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::track_gate::TimeTrialTrackGate;

#[uclass(parent = GameModeBase)]
pub struct TimeTrialGameMode {
    #[uproperty(EditAnywhere, category = "Time Trial")]
    finish_tag: FName,

    #[uproperty(EditAnywhere, category = "Time Trial", default = 3)]
    laps: i32,

    #[uproperty]
    finish_line_marker: UObjectRef<TimeTrialTrackGate>,

    #[uproperty(EditDefaultsOnly, category = "Local Multiplayer", default = 1)]
    number_of_local_players: i32,

    current_player_start_assignment: i32,
}

#[uclass_impl]
impl TimeTrialGameMode {
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        let world = self.as_ref().upcast_to();

        let gates = GameplayStatics::get_all_actors_of_class_with_tag(
            world,
            SubclassOf::<TimeTrialTrackGate>::base().upcast_to(),
            self.finish_tag().handle(),
        );

        if let Some(finish_line) = gates.first() {
            self.set_finish_line_marker(finish_line.cast().unwrap_or_default());
        }

        for _ in 2..=self.number_of_local_players() {
            GameplayStatics::create_player(world, Some(-1), Some(true));
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

impl TimeTrialGameMode {
    pub fn get_finish_line(&self) -> UObjectRef<TimeTrialTrackGate> {
        self.finish_line_marker()
    }
}
