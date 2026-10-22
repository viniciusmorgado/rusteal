use bindings::engine::ActorExt;
use bindings::plugins::rusteal_probe::{ProbeTarget, ProbeTargetExt};
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

pub fn game_uses_a_plugin_class_through_its_bindings(context: &Context) -> Outcome {
    Outcome::Done((|| {
        let e = |e| format!("{e:?}");
        let transform = FTransform::from_transform(Transform::IDENTITY);
        let target = context
            .world
            .spawn_actor::<ProbeTarget>(&transform)
            .map_err(e)?;
        let probe = target.checked().map_err(e)?;

        probe.set_charge(4);
        probe.set_label("before".to_string());

        let total = probe.add_charge(6).map_err(e)?;
        let times = probe.times_charged().map_err(e)?;
        let description = probe.describe().map_err(e)?;
        probe.rename("after".to_string()).map_err(e)?;
        let label = probe.label();
        let charge = probe.charge();

        let is_rust = <ProbeTarget as rusteal_runtime::runtime::UeClass>::RUST_TYPE_ID
            == rusteal_runtime::runtime::type_id("rusteal_probe::probe::ProbeTarget");

        probe.k2_destroy_actor();

        ensure(
            total == 10
                && charge == 10
                && times == 1
                && description == "probe at 10"
                && label == "after"
                && is_rust,
            format!(
                "AddCharge gave {total}, Charge {charge}, TimesCharged {times}, Describe \
                 {description:?}, Label {label:?}, the type id matched: {is_rust}"
            ),
        )
    })())
}
