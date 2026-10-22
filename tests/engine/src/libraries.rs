use bindings::engine::ActorExt;
use bindings::prelude::*;
use rusteal_runtime::runtime::{DynamicCall, Transform, UObjectRef};

use crate::runner::{Context, Outcome};

pub const PROBE_TARGET: &str = "/Script/RustealProbe.ProbeTarget";

fn ensure(condition: bool, what: impl Into<String>) -> Result<(), String> {
    if condition { Ok(()) } else { Err(what.into()) }
}

fn add_charge(target: &UObjectRef<Actor>, amount: i32) -> Result<i32, String> {
    let e = |e| format!("{e:?}");
    let mut call = DynamicCall::new(target, "AddCharge").map_err(e)?;
    call.set("Amount", amount).map_err(e)?;
    call.call().map_err(e)?.get("ReturnValue").map_err(e)
}

pub fn plugin_class_is_reached_by_reflection(context: &Context) -> Outcome {
    Outcome::Done((|| {
        let e = |e| format!("{e:?}");
        let class = load_class::<Actor>(PROBE_TARGET)
            .map_err(|e| format!("the plugin's class {PROBE_TARGET} is not there: {e:?}"))?;
        let transform = FTransform::from_transform(Transform::IDENTITY);
        let target = context
            .world
            .spawn_actor_of_class::<Actor>(class, &transform)
            .map_err(e)?;

        let first = add_charge(&target, 5)?;
        let second = add_charge(&target, 3)?;

        let times: i32 = DynamicCall::new(&target, "TimesCharged")
            .map_err(e)?
            .call()
            .map_err(e)?
            .get("ReturnValue")
            .map_err(e)?;

        let description: String = DynamicCall::new(&target, "Describe")
            .map_err(e)?
            .call()
            .map_err(e)?
            .value("ReturnValue")
            .map_err(e)?;

        target.checked().map_err(e)?.k2_destroy_actor();

        ensure(
            first == 5 && second == 8 && times == 2 && description == "probe at 8",
            format!(
                "AddCharge gave {first} then {second}, TimesCharged {times}, Describe {description:?}"
            ),
        )
    })())
}
