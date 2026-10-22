use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use bindings::engine::{Actor, ActorExt, GameplayStatics, SceneComponent};
use bindings::prelude::*;
use rusteal_runtime::runtime::{RustealResult, SubclassOf, Transform, UeArray};
use rusteal_runtime::{uclass, uclass_impl};

use super::{NetWorld, Role, all_players_connected};

const COUNT: i32 = 7;
const LABEL: &str = "replicated";
const VALUES: [i32; 3] = [1, 2, 3];
const SECRET: i32 = 42;

static SPAWNED: AtomicBool = AtomicBool::new(false);
static COUNT_NOTIFIES: Mutex<Option<HashMap<usize, (i32, i32)>>> = Mutex::new(None);

#[uclass(parent = Actor)]
pub struct NetProbe {
    #[component(root)]
    root: SceneComponent,

    #[uproperty(ReplicatedUsing = on_rep_count)]
    count: i32,

    #[uproperty(Replicated)]
    label: String,

    #[uproperty(Replicated)]
    values: UeArray<i32>,

    #[uproperty(Replicated, replication = "OwnerOnly")]
    secret: i32,
}

#[uclass_impl]
impl NetProbe {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let probe = self.as_ref().checked()?;
        probe.set_replicates(true);
        probe.set_always_relevant(true);
        Ok(())
    }

    #[ufunction]
    fn on_rep_count(&mut self, old_count: i32) {
        let viewer = GameplayStatics::get_player_controller(self.as_ref().upcast_to(), 0)
            .raw()
            .0 as usize;

        if let Ok(mut notifies) = COUNT_NOTIFIES.lock() {
            notifies
                .get_or_insert_with(HashMap::new)
                .insert(viewer, (old_count, self.count()));
        }
    }
}

fn spawn_probe(net: &NetWorld) -> Result<(), String> {
    let e = |e| format!("{e:?}");
    let owner = GameplayStatics::get_player_controller(net.context, 0).upcast_to::<Actor>();
    let transform = FTransform::from_transform(Transform::IDENTITY);
    let spawned = net
        .world
        .spawn_actor_with_owner::<NetProbe>(&transform, &owner)
        .map_err(e)?;

    let probe = NetProbe::from_obj(spawned).map_err(e)?;
    probe.set_count(COUNT);
    probe.set_label(LABEL);
    probe.set_secret(SECRET);

    for value in VALUES {
        probe.values().push(&value).map_err(e)?;
    }

    Ok(())
}

fn seen_by_client(net: &NetWorld) -> Option<Result<(), String>> {
    let e = |e| format!("{e:?}");
    let found = GameplayStatics::get_all_actors_of_class(
        net.context,
        SubclassOf::<NetProbe>::base().upcast_to::<Actor>(),
    );
    let actor = *found.first()?;
    let probe = NetProbe::from_obj(actor).ok()?;

    if probe.count() != COUNT {
        return None;
    }

    let local = GameplayStatics::get_player_controller(net.context, 0);
    let owned = actor
        .checked()
        .is_ok_and(|actor| actor.get_owner().raw() == local.raw());
    let notified = COUNT_NOTIFIES
        .lock()
        .ok()
        .and_then(|notifies| notifies.as_ref()?.get(&(local.raw().0 as usize)).copied());

    let values = match probe.values().to_vec() {
        Ok(values) => values,
        Err(error) => return Some(Err(e(error))),
    };
    let expected_secret = if owned { SECRET } else { 0 };

    let problems: Vec<String> = [
        (probe.label() != LABEL).then(|| format!("label {:?}", probe.label())),
        (values != VALUES).then(|| format!("values {values:?}")),
        (probe.secret() != expected_secret).then(|| {
            format!(
                "the owner-only value is {} on a client that {} the probe",
                probe.secret(),
                if owned { "owns" } else { "does not own" }
            )
        }),
        (notified != Some((0, COUNT))).then(|| format!("on_rep_count saw {notified:?}")),
    ]
    .into_iter()
    .flatten()
    .collect();

    Some(if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join(", "))
    })
}

pub fn replicated_properties_reach_the_clients(net: &NetWorld) -> Option<Result<(), String>> {
    match net.role {
        Role::Server => {
            if !all_players_connected(net) {
                return None;
            }

            if SPAWNED.swap(true, Ordering::SeqCst) {
                return Some(Ok(()));
            }

            Some(spawn_probe(net))
        }
        Role::Client(_) => seen_by_client(net),
    }
}
