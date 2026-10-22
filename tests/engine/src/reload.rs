use bindings::core_ue::Object;
use bindings::engine::{GameplayStatics, KismetMathLibrary, World};
use bindings::manual::world_ext::new_object;
use bindings::plugins::rusteal_probe::{ProbeRecord, ProbeRecordExt};
use bindings::prelude::*;
use rusteal_runtime::runtime::{SubclassOf, Transform, UObjectRef, console};
use rusteal_runtime::{uclass, uclass_impl};

const PROBE_RECORD: &str = "/Script/RustealProbe.ProbeRecord";

#[uclass(parent = ProbeRecord)]
pub struct GameRecord {
    #[uproperty]
    seen_total: i32,
}

#[uclass_impl]
impl GameRecord {
    #[ufunction(Override)]
    fn on_charged(&mut self, total: i32) {
        self.set_seen_total(total);
    }
}

#[uclass(parent = Actor)]
pub struct ReloadHolder {
    #[uproperty]
    record: UObjectRef<GameRecord>,
}

fn results(lines: &[String]) {
    if let Ok(path) = std::env::var("RUSTEAL_TEST_RESULTS") {
        let mut text = lines.join("\n");
        text.push_str("\nDONE\n");
        let _ = std::fs::write(path, text);
    }
}

fn prepare(world: UObjectRef<World>) -> Result<(), String> {
    let e = |e| format!("{e:?}");
    let transform = FTransform::from_transform(Transform::IDENTITY);
    let holder = world.spawn_actor::<ReloadHolder>(&transform).map_err(e)?;
    let record = new_object::<GameRecord>(&holder).map_err(e)?;

    let probe = record.checked().map_err(e)?;
    probe.set_charge(2);
    probe.add_charge(0).map_err(e)?;

    ReloadHolder::from_obj(holder)
        .map_err(e)?
        .set_record(record);

    let variant = std::env::var("RUSTEAL_RELOAD_VARIANT").map_err(|_| "no variant library")?;
    let target = std::env::var("RUSTEAL_RELOAD_TARGET").map_err(|_| "no plugin library")?;
    let staged = format!("{target}.next");
    std::fs::copy(&variant, &staged).map_err(|e| format!("cannot copy {variant}: {e}"))?;
    std::fs::rename(&staged, &target).map_err(|e| format!("cannot replace {target}: {e}"))?;

    Ok(())
}

fn verify(world: UObjectRef<World>) -> Result<(), String> {
    let e = |e| format!("{e:?}");
    let context: UObjectRef<Object> = world.upcast_to();
    let holders = GameplayStatics::get_all_actors_of_class(
        context,
        SubclassOf::<ReloadHolder>::base().upcast_to(),
    );

    let holder = holders
        .first()
        .copied()
        .ok_or("no ReloadHolder after the reload")?;
    let record = ReloadHolder::from_obj(holder).map_err(e)?.record();
    let probe = record
        .cast::<ProbeRecord>()
        .map_err(e)?
        .checked()
        .map_err(e)?;

    let class = GameplayStatics::get_object_class(record.upcast_to());
    let new_parent = rusteal_runtime::runtime::reflected::class_at(PROBE_RECORD);
    let reparented = KismetMathLibrary::class_is_child_of(
        class,
        SubclassOf::from_class(new_parent).map_err(e)?,
    );

    let kept = probe.charge();
    let total = probe.add_charge(3).map_err(e)?;
    let times = probe.times_charged().map_err(e)?;
    let seen = GameRecord::from_obj(record).map_err(e)?.seen_total();

    if reparented && kept == 2 && total == 5 && times == 101 && seen == 5 {
        Ok(())
    } else {
        Err(format!(
            "the object's class derives from the new ProbeRecord: {reparented}; Charge kept \
             {kept}; AddCharge gave {total}; the plugin's new code counted {times}; the game saw \
             {seen}"
        ))
    }
}

pub fn register() {
    let prepared = console::register_command(
        "rusteal_tests.reload_prepare",
        "Creates a game class of a plugin parent and replaces the plugin's library.",
        |_, context| {
            let outcome = context
                .world::<World>()
                .map_err(|e| format!("{e:?}"))
                .and_then(prepare);

            if let Err(reason) = outcome {
                results(&[format!("FAIL chained_reload_prepare: {reason}")]);
            }
        },
    );

    let verified = console::register_command(
        "rusteal_tests.reload_verify",
        "Checks the game class after its plugin parent was reloaded.",
        |_, context| {
            let outcome = context
                .world::<World>()
                .map_err(|e| format!("{e:?}"))
                .and_then(verify);

            let line = match outcome {
                Ok(()) => "PASS chained_reload_rebuilds_the_derived_game_class".to_string(),
                Err(reason) => {
                    format!("FAIL chained_reload_rebuilds_the_derived_game_class: {reason}")
                }
            };

            if std::env::var("RUSTEAL_TEST_RESULTS")
                .ok()
                .and_then(|path| std::fs::read_to_string(path).ok())
                .is_none_or(|text| !text.contains("FAIL chained_reload_prepare"))
            {
                results(&[line]);
            }
        },
    );

    for registered in [prepared, verified] {
        if let Err(error) = registered {
            log::error!("[RustealTests] cannot register a reload command: {error:?}");
        }
    }
}
