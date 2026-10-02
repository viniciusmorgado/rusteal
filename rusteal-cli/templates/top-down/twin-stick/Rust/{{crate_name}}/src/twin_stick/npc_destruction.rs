// TwinStickNPCDestruction: the TwinStick variant's `ATwinStickNPCDestruction`
// in Rust, a destruction proxy that replaces an NPC when it is destroyed:
// its Blueprint child `BP_TwinStickNPCDestruction` plays the effects, apart
// from gameplay.

use bindings::engine::Actor;
use rusteal_runtime::uclass;

#[uclass(parent = Actor)]
pub struct TwinStickNPCDestruction {}
