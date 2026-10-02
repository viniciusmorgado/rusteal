// What the SideScrolling variant's `ISideScrollingInteractable` C++ interface
// does: actors the player can interact with. Only Rust classes implement it,
// so it is a Rust trait, and `interact` finds which class an actor is.

use bindings::engine::Actor;
use rusteal_runtime::runtime::UObjectRef;

use super::moving_platform::SideScrollingMovingPlatform;
use super::npc::SideScrollingNPC;

/// Simple interface to allow Actors to interact without having knowledge of their internal implementation.
pub trait Interactable {
    /// Triggers an interaction by the provided Actor
    fn interaction(&mut self, interactor: UObjectRef<Actor>);
}

/// `Cast<ISideScrollingInteractable>(Actor)->Interaction(Interactor)`: interact
/// with `target` if it is interactable; `false` when it is not.
pub fn interact(target: UObjectRef<Actor>, interactor: UObjectRef<Actor>) -> bool {
    if let Ok(mut platform) = SideScrollingMovingPlatform::from_obj(target) {
        platform.interaction(interactor);
        true
    } else if let Ok(mut npc) = SideScrollingNPC::from_obj(target) {
        npc.interaction(interactor);
        true
    } else {
        false
    }
}
