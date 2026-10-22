use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use bindings::core_ue::Object;
use bindings::engine::{Actor, ActorExt, GameplayStatics, SceneComponent};
use bindings::prelude::*;
use rusteal_runtime::runtime::{RustealResult, SubclassOf, Transform, UObjectRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::{CLIENTS, NetWorld, Role, all_players_connected};

const ADDED: i32 = 5;
const NOTE: i32 = 11;
const PING: i32 = 13;

#[derive(Default)]
struct Calls {
    seen: HashSet<usize>,
    notes: HashMap<usize, i32>,
    pings: HashMap<usize, i32>,
    refused_sent: bool,
}

static SPAWNED: AtomicBool = AtomicBool::new(false);
static ADD_SENT: AtomicBool = AtomicBool::new(false);
static NOTE_SENT: AtomicBool = AtomicBool::new(false);
static PING_SENT: AtomicBool = AtomicBool::new(false);
static CALLS: Mutex<Option<Calls>> = Mutex::new(None);

fn viewer(context: UObjectRef<Object>) -> usize {
    GameplayStatics::get_player_controller(context, 0)
        .raw()
        .0 as usize
}

fn with_calls<R>(f: impl FnOnce(&mut Calls) -> R) -> Option<R> {
    let mut calls = CALLS.lock().ok()?;
    Some(f(calls.get_or_insert_with(Calls::default)))
}

#[uclass(parent = Actor)]
pub struct RpcProbe {
    #[component(root)]
    root: SceneComponent,

    #[uproperty(Replicated)]
    total: i32,
}

#[uclass_impl]
impl RpcProbe {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let probe = self.as_ref().checked()?;
        probe.set_replicates(true);
        probe.set_always_relevant(true);
        Ok(())
    }

    #[ufunction(Server, Reliable, WithValidation = valid_amount)]
    fn server_add(&mut self, amount: i32) {
        self.set_total(self.total() + amount);
    }

    fn valid_amount(&self, amount: i32) -> bool {
        amount >= 0
    }

    #[ufunction(Client, Reliable)]
    fn client_note(&mut self, value: i32) {
        let viewer = viewer(self.as_ref().upcast_to());
        with_calls(|calls| calls.notes.insert(viewer, value));
    }

    #[ufunction(NetMulticast, Reliable)]
    fn multicast_ping(&mut self, value: i32) {
        let viewer = viewer(self.as_ref().upcast_to());
        with_calls(|calls| calls.pings.insert(viewer, value));
    }
}

fn probe(net: &NetWorld) -> Option<RpcProbe> {
    let found = GameplayStatics::get_all_actors_of_class(
        net.context,
        SubclassOf::<RpcProbe>::base().upcast_to::<Actor>(),
    );

    RpcProbe::from_obj(*found.first()?).ok()
}

fn owns(net: &NetWorld, probe: &RpcProbe) -> bool {
    let local = GameplayStatics::get_player_controller(net.context, 0).raw();

    probe
        .as_ref()
        .upcast_to::<Actor>()
        .checked()
        .is_ok_and(|actor| actor.get_owner().raw() == local)
}

fn spawn(net: &NetWorld) -> Result<(), String> {
    let owner = GameplayStatics::get_player_controller(net.context, 0).upcast_to::<Actor>();
    let transform = FTransform::from_transform(Transform::IDENTITY);

    net.world
        .spawn_actor_with_owner::<RpcProbe>(&transform, &owner)
        .map(|_| ())
        .map_err(|e| format!("{e:?}"))
}

pub fn server_function_runs_on_the_server(net: &NetWorld) -> Option<Result<(), String>> {
    match net.role {
        Role::Server => {
            if !all_players_connected(net) {
                return None;
            }

            if !SPAWNED.swap(true, Ordering::SeqCst) {
                return Some(spawn(net)).filter(Result::is_err);
            }

            (probe(net)?.total() == ADDED).then_some(Ok(()))
        }
        Role::Client(_) => {
            let probe = probe(net)?;
            let viewer = viewer(net.context);
            with_calls(|calls| calls.seen.insert(viewer));

            if owns(net, &probe) && !ADD_SENT.swap(true, Ordering::SeqCst) {
                probe.server_add(ADDED);
            }

            (probe.total() == ADDED).then_some(Ok(()))
        }
    }
}

pub fn client_function_runs_on_its_owner_only(net: &NetWorld) -> Option<Result<(), String>> {
    match net.role {
        Role::Server => {
            let probe = probe(net)?;
            let everyone_saw_it = with_calls(|calls| calls.seen.len() == CLIENTS)?;

            if probe.total() != ADDED || !everyone_saw_it {
                return None;
            }

            if !NOTE_SENT.swap(true, Ordering::SeqCst) {
                probe.client_note(NOTE);
            }

            if !PING_SENT.swap(true, Ordering::SeqCst) {
                probe.multicast_ping(PING);
            }

            Some(Ok(()))
        }
        Role::Client(_) => {
            let probe = probe(net)?;
            let viewer = viewer(net.context);
            let (note, pinged) =
                with_calls(|calls| (calls.notes.get(&viewer).copied(), calls.pings.contains_key(&viewer)))?;

            match (owns(net, &probe), note) {
                (true, Some(NOTE)) => Some(Ok(())),
                (true, Some(other)) => Some(Err(format!("the owner got {other}"))),
                (true, None) => None,
                (false, Some(other)) => Some(Err(format!("a client that does not own the probe got {other}"))),
                (false, None) => pinged.then_some(Ok(())),
            }
        }
    }
}

pub fn multicast_runs_everywhere(net: &NetWorld) -> Option<Result<(), String>> {
    let viewer = viewer(net.context);
    let ping = with_calls(|calls| calls.pings.get(&viewer).copied())??;

    Some(if ping == PING {
        Ok(())
    } else {
        Err(format!("got {ping}"))
    })
}

pub fn validation_refuses_a_bad_call(net: &NetWorld) -> Option<Result<(), String>> {
    match net.role {
        Role::Server => {
            let probe = probe(net)?;
            let sent = with_calls(|calls| calls.refused_sent)?;

            if !sent || GameplayStatics::get_num_player_controllers(net.context) == CLIENTS as i32 {
                return None;
            }

            Some(if probe.total() == ADDED {
                Ok(())
            } else {
                Err(format!("the refused call ran: the total is {}", probe.total()))
            })
        }
        Role::Client(_) => {
            let probe = probe(net)?;

            if !owns(net, &probe) {
                return Some(Ok(()));
            }

            let viewer = viewer(net.context);
            let ready = with_calls(|calls| {
                calls.notes.contains_key(&viewer) && calls.pings.contains_key(&viewer)
            })?;

            if !ready {
                return None;
            }

            probe.server_add(-1);
            with_calls(|calls| calls.refused_sent = true);
            Some(Ok(()))
        }
    }
}
