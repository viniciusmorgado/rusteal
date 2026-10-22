use std::hint::black_box;
use std::time::Instant;

use bindings::core_ue::Object;
use bindings::engine::{ActorExt, KismetMathLibrary, SceneComponent};
use bindings::prelude::*;
use rusteal_runtime::runtime::reflected;
use rusteal_runtime::runtime::{DynamicCall, Transform, UObjectRef, UeArray, UeClass};
use rusteal_runtime::{uclass, uclass_impl};

use crate::runner::{Context, Outcome};

const CALLS: u32 = 100_000;
const ELEMENTS: usize = 100_000;

#[uclass(parent = Actor)]
pub struct PerfProbe {
    #[component(root)]
    root: SceneComponent,

    #[uproperty]
    count: i32,

    #[uproperty]
    samples: UeArray<i32>,
}

#[uclass_impl]
impl PerfProbe {}

fn nanoseconds_per(start: Instant, count: u32) -> f64 {
    start.elapsed().as_nanos() as f64 / f64::from(count)
}

fn measure(context: &Context) -> Result<Vec<String>, String> {
    let e = |e| format!("{e:?}");
    let mut lines = Vec::new();

    let start = Instant::now();
    let mut sum = 0i32;
    for n in 0..CALLS {
        sum = black_box(sum.wrapping_add(n as i32));
    }
    lines.push(format!("rust_add_baseline {:.1} ns", nanoseconds_per(start, CALLS)));

    let start = Instant::now();
    for n in 0..CALLS {
        sum = KismetMathLibrary::add_int_int(black_box(sum), Some(n as i32 & 7));
    }
    lines.push(format!("binding_call {:.1} ns", nanoseconds_per(start, CALLS)));

    let library = UObjectRef::<Object>::from_handle(
        reflected::default_object(KismetMathLibrary::static_class()).map_err(e)?,
    )
    .map_err(e)?;

    let start = Instant::now();
    for n in 0..CALLS {
        let mut call = DynamicCall::new(&library, "Add_IntInt").map_err(e)?;
        call.set("A", black_box(sum)).map_err(e)?;
        call.set("B", n as i32 & 7).map_err(e)?;
        sum = call.call().map_err(e)?.get("ReturnValue").map_err(e)?;
    }
    lines.push(format!(
        "reflection_call (ProcessEvent, the Blueprint path) {:.1} ns",
        nanoseconds_per(start, CALLS)
    ));

    let transform = FTransform::from_transform(Transform::IDENTITY);
    let actor = context
        .world
        .spawn_actor::<PerfProbe>(&transform)
        .map_err(e)?;
    let probe = PerfProbe::from_obj(actor).map_err(e)?;

    let start = Instant::now();
    for n in 0..CALLS {
        probe.set_count(black_box(probe.count().wrapping_add(n as i32)));
    }
    lines.push(format!(
        "property_get_and_set {:.1} ns",
        nanoseconds_per(start, CALLS)
    ));

    let values: Vec<i32> = (0..ELEMENTS as i32).collect();
    let start = Instant::now();
    probe.samples().set_all(&values).map_err(e)?;
    let copied = probe.samples().to_vec().map_err(e)?;
    let bulk = start.elapsed();

    if copied != values {
        return Err("the bulk copy lost elements".to_string());
    }

    lines.push(format!(
        "array_bulk_copy_in_and_out of {ELEMENTS} i32 {:.1} us",
        bulk.as_nanos() as f64 / 1000.0
    ));

    let start = Instant::now();
    let array = probe.samples();
    let mut total = 0i64;
    for index in 0..ELEMENTS {
        total += i64::from(array.get(index).map_err(e)?);
    }
    lines.push(format!(
        "array_get_per_element {:.1} ns",
        start.elapsed().as_nanos() as f64 / ELEMENTS as f64
    ));

    black_box((sum, total));
    actor.checked().map_err(e)?.k2_destroy_actor();

    Ok(lines)
}

pub fn call_overhead_is_measured(context: &Context) -> Outcome {
    Outcome::Done(measure(context).map(|lines| {
        for line in &lines {
            log::info!("[RustealPerf] {line}");
        }

        if let Ok(path) = std::env::var("RUSTEAL_PERF_RESULTS") {
            let _ = std::fs::write(path, lines.join("\n") + "\n");
        }
    }))
}
