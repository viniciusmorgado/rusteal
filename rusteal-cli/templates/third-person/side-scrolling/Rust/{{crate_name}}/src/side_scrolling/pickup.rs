// SideScrollingPickup: the SideScrolling variant's `ASideScrollingPickup` in
// Rust. The player collects it by touching it; its Blueprint child
// `BP_SideScrollingPickup` plays the pickup effect and destroys it
// (`BP_OnPickedUp`).

use bindings::engine::{
    Actor, ActorExt, Character, ECollisionChannel, ECollisionEnabled, ECollisionResponse,
    GameplayStatics, PawnExt, PrimitiveComponentExt, SceneComponent, SphereComponent,
    SphereComponentExt,
};
use rusteal_runtime::runtime::{LOG_WARNING, RustealResult, UObjectRef, ulog};
use rusteal_runtime::{uclass, uclass_impl};

use super::game_mode::SideScrollingGameMode;
use super::model;

#[uclass(parent = Actor)]
pub struct SideScrollingPickup {
    #[component(root)]
    root: SceneComponent,

    /// Pickup bounding sphere
    #[component(attach = "root", name = "Collision")]
    sphere: SphereComponent,
}

#[uclass_impl]
impl SideScrollingPickup {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        // create the bounding sphere
        let sphere = self.sphere()?.checked()?;
        sphere.set_sphere_radius(model::PICKUP_SPHERE_RADIUS, Some(false));
        sphere.set_collision_object_type(ECollisionChannel::ECC_WorldDynamic);
        sphere.set_collision_enabled(ECollisionEnabled::QueryOnly);
        sphere.set_collision_response_to_all_channels(ECollisionResponse::ECR_Ignore);
        sphere.set_collision_response_to_channel(ECollisionChannel::ECC_Pawn, ECollisionResponse::ECR_Overlap);
        Ok(())
    }

    /// Add the overlap handler, as the C++ constructor does.
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        let me = self.as_ref();
        if let Err(e) = me.checked().and_then(|actor| actor.on_actor_begin_overlap().add_ufunction(&me, "BeginOverlap")) {
            ulog!(LOG_WARNING, "[SideScrolling] pickup overlap: {e}");
        }
    }

    /// Handles pickup collision
    #[ufunction(BlueprintCallable)]
    fn begin_overlap(&mut self, _overlapped_actor: UObjectRef<Actor>, other_actor: UObjectRef<Actor>) {
        if let Err(e) = self.pick_up(other_actor) {
            ulog!(LOG_WARNING, "[SideScrolling] pickup failed: {e}");
        }
    }

    /// Passes control to Blueprint to animate the pickup and destroy it
    #[ufunction(BlueprintImplementableEvent, name = "BP_OnPickedUp")]
    fn bp_on_picked_up(&self) {}
}

impl SideScrollingPickup {
    fn pick_up(&mut self, other_actor: UObjectRef<Actor>) -> RustealResult<()> {
        // have we collided against a character?
        let Ok(overlapped_character) = other_actor.cast::<Character>() else {
            return Ok(());
        };
        // is this the player character?
        if !overlapped_character.checked()?.is_player_controlled() {
            return Ok(());
        }
        // get the game mode
        let Ok(mut game_mode) =
            SideScrollingGameMode::from_obj(GameplayStatics::get_game_mode(self.as_ref().upcast_to()))
        else {
            return Ok(());
        };
        // tell the game mode to process a pickup
        game_mode.process_pickup()?;
        // disable collision so we don't get picked up again
        self.as_ref().checked()?.set_actor_enable_collision(false);
        // Call the BP handler. It will be responsible for destroying the pickup
        self.bp_on_picked_up();
        Ok(())
    }
}
