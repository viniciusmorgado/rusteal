// SideScrollingGameMode: the SideScrolling variant's `ASideScrollingGameMode`
// in Rust. Spawns the game UI and the extra local players, assigns each player
// its own PlayerStart, and counts the pickups the player collects. Its
// Blueprint child `BP_SideScrollingGameMode` sets the UI class and the pawn,
// controller and camera classes.

use bindings::engine::{Actor, Controller, GameModeBase, GameplayStatics, KismetMathLibrary, PlayerStart};
use bindings::prelude::*;
use bindings::umg::UserWidgetExt;
use rusteal_runtime::runtime::{
    FName, LOG_WARNING, RustealResult, SubclassOf, UObjectRef, ulog,
};
use rusteal_runtime::{uclass, uclass_impl};

use super::ui::SideScrollingUI;

#[uclass(parent = GameModeBase)]
pub struct SideScrollingGameMode {
    /// Class of UI widget to spawn when the game starts
    #[uproperty(EditAnywhere)]
    user_interface_class: SubclassOf<SideScrollingUI>,

    /// User interface widget for the game
    #[uproperty(BlueprintReadOnly)]
    user_interface: UObjectRef<SideScrollingUI>,

    /// Number of pickups collected by the player
    #[uproperty(BlueprintReadOnly)]
    pickups_collected: i32,

    /// Determines how many local players should be spawned on game start
    #[uproperty(EditDefaultsOnly, default = 1)]
    number_of_local_players: i32,

    /// Used to assign players to different PlayerStarts in the level
    current_player_start_assignment: i32,
}

#[uclass_impl]
impl SideScrollingGameMode {
    /// Initialization
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let Err(e) = self.create_user_interface() {
            ulog!(LOG_WARNING, "[SideScrolling] cannot create the game UI: {e}");
        }

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

impl SideScrollingGameMode {
    /// Receives an interaction event from another actor
    pub fn process_pickup(&mut self) -> RustealResult<()> {
        // increment the pickups counter
        let pickups = self.pickups_collected() + 1;
        self.set_pickups_collected(pickups);

        let user_interface = self.user_interface();
        if user_interface.is_valid() {
            // if this is the first pickup we collect, show the UI
            if pickups == 1 {
                user_interface.upcast_to::<bindings::umg::UserWidget>().checked()?.add_to_viewport(Some(0));
            }
            // update the pickups counter on the UI
            SideScrollingUI::from_obj(user_interface)?.update_pickups(pickups);
        }
        Ok(())
    }

    /// The game UI, owned by the first player.
    fn create_user_interface(&mut self) -> RustealResult<()> {
        let owning_player = GameplayStatics::get_player_controller(self.as_ref().upcast_to(), 0);
        let user_interface = create_widget_of_class(&owning_player, self.user_interface_class())?;
        self.set_user_interface(user_interface);
        Ok(())
    }
}
