use bindings::engine::{
    Actor, Controller, GameModeBase, GameplayStatics, KismetMathLibrary, PlayerStart,
};
use rusteal_runtime::runtime::{FName, SubclassOf, UObjectRef};
use rusteal_runtime::{uclass, uclass_impl};

#[uclass(parent = GameModeBase)]
pub struct HorrorGameMode {
    #[uproperty(EditDefaultsOnly, default = 1)]
    number_of_local_players: i32,

    current_player_start_assignment: i32,
}

#[uclass_impl]
impl HorrorGameMode {
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        for _ in 2..=self.number_of_local_players() {
            GameplayStatics::create_player(self.as_ref().upcast_to(), Some(-1), Some(true));
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
