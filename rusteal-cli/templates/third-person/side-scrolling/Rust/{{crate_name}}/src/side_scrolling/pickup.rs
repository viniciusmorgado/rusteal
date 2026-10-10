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

    #[component(attach = "root", name = "Collision")]
    sphere: SphereComponent,
}

#[uclass_impl]
impl SideScrollingPickup {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let sphere = self.sphere()?.checked()?;
        sphere.set_sphere_radius(model::PICKUP_SPHERE_RADIUS, Some(false));
        sphere.set_collision_object_type(ECollisionChannel::ECC_WorldDynamic);
        sphere.set_collision_enabled(ECollisionEnabled::QueryOnly);
        sphere.set_collision_response_to_all_channels(ECollisionResponse::ECR_Ignore);

        sphere.set_collision_response_to_channel(
            ECollisionChannel::ECC_Pawn,
            ECollisionResponse::ECR_Overlap,
        );

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        let me = self.as_ref();

        if let Err(e) = me.checked().and_then(|actor| {
            actor
                .on_actor_begin_overlap()
                .add_ufunction(&me, "BeginOverlap")
        }) {
            ulog!(LOG_WARNING, "[SideScrolling] pickup overlap: {e}");
        }
    }

    #[ufunction(BlueprintCallable)]
    fn begin_overlap(
        &mut self,
        _overlapped_actor: UObjectRef<Actor>,
        other_actor: UObjectRef<Actor>,
    ) {
        if let Err(e) = self.pick_up(other_actor) {
            ulog!(LOG_WARNING, "[SideScrolling] pickup failed: {e}");
        }
    }

    #[ufunction(BlueprintImplementableEvent, name = "BP_OnPickedUp")]
    fn bp_on_picked_up(&self) {}
}

impl SideScrollingPickup {
    fn pick_up(&mut self, other_actor: UObjectRef<Actor>) -> RustealResult<()> {
        let Ok(overlapped_character) = other_actor.cast::<Character>() else {
            return Ok(());
        };

        if !overlapped_character.checked()?.is_player_controlled() {
            return Ok(());
        }

        let Ok(mut game_mode) = SideScrollingGameMode::from_obj(GameplayStatics::get_game_mode(
            self.as_ref().upcast_to(),
        )) else {
            return Ok(());
        };

        game_mode.process_pickup()?;

        self.as_ref().checked()?.set_actor_enable_collision(false);

        self.bp_on_picked_up();

        Ok(())
    }
}
