use bindings::engine::Actor;
use rusteal_runtime::runtime::UObjectRef;

use super::moving_platform::SideScrollingMovingPlatform;
use super::npc::SideScrollingNPC;

pub trait Interactable {
    fn interaction(&mut self, interactor: UObjectRef<Actor>);
}

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
