use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use bindings::core_ue::Object;
use bindings::engine::{ControllerExt, GameplayStatics, KismetSystemLibrary};
use bindings::rusteal::{RustealWorldSubsystem, RustealWorldSubsystemExt};
use rusteal_runtime::runtime::reflected::{self, PropertySlot};
use rusteal_runtime::runtime::{RustealResult, UObjectRef, console};
use rusteal_runtime::{uclass, uclass_impl};

const RESULTS: &str = "RUSTEAL_NET_TEST_RESULTS";
const CLIENTS: usize = 2;
const TIMEOUT: Duration = Duration::from_secs(60);

const PIE_CLIENT: u8 = 2;

static PLAY_NET_MODE: PropertySlot = PropertySlot::new("PlayNetMode");
static RUN_UNDER_ONE_PROCESS: PropertySlot = PropertySlot::new("RunUnderOneProcess");
static PLAY_NUMBER_OF_CLIENTS: PropertySlot = PropertySlot::new("PlayNumberOfClients");

fn configure_play_in_editor() {
    let class = reflected::class_at("/Script/UnrealEd.LevelEditorPlaySettings");

    match reflected::default_object(class) {
        Ok(settings) => {
            PLAY_NET_MODE.set_copy(settings, class, PIE_CLIENT);
            RUN_UNDER_ONE_PROCESS.set(settings, class, &true);
            PLAY_NUMBER_OF_CLIENTS.set(settings, class, &(CLIENTS as i32));
        }
        Err(error) => log::error!("[RustealNetTests] no play settings: {error:?}"),
    }
}

pub fn register() {
    if let Err(error) = console::register_command(
        "rusteal_tests.net_configure_pie",
        "Makes Play In Editor run a dedicated server and two clients in this process.",
        |_, _| configure_play_in_editor(),
    ) {
        log::error!("[RustealNetTests] cannot register rusteal_tests.net_configure_pie: {error:?}");
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Role {
    Server,
    Client(usize),
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Server => write!(f, "server"),
            Role::Client(index) => write!(f, "client {index}"),
        }
    }
}

pub struct NetCheck {
    pub name: &'static str,
    pub check: fn(Role, &UObjectRef<Object>) -> Option<Result<(), String>>,
}

#[derive(Default)]
struct NetRun {
    started: Option<Instant>,
    clients: usize,
    outcomes: BTreeMap<(&'static str, Role), Result<(), String>>,
    finished: bool,
}

static RUN: Mutex<Option<NetRun>> = Mutex::new(None);

fn checks() -> &'static [NetCheck] {
    &[NetCheck {
        name: "dedicated_server_sees_two_clients",
        check: players_are_connected,
    }]
}

fn players_are_connected(role: Role, world: &UObjectRef<Object>) -> Option<Result<(), String>> {
    let players = GameplayStatics::get_num_player_controllers(*world);

    match role {
        Role::Server if players == CLIENTS as i32 => Some(Ok(())),
        Role::Client(_) if players == 1 => {
            let local = GameplayStatics::get_player_controller(*world, 0)
                .checked()
                .is_ok_and(|controller| controller.is_local_controller());

            Some(if local {
                Ok(())
            } else {
                Err("its player controller is not local".to_string())
            })
        }
        _ => None,
    }
}

fn finish(run: &mut NetRun) {
    run.finished = true;

    let roles = std::iter::once(Role::Server).chain((0..CLIENTS).map(Role::Client));
    let mut lines: Vec<String> = run
        .outcomes
        .iter()
        .filter(|((name, _), _)| *name == "net_play_in_editor")
        .filter_map(|(_, outcome)| outcome.as_ref().err())
        .map(|reason| format!("FAIL net_play_in_editor: {reason}"))
        .collect();

    for check in checks() {
        let failures: Vec<String> = roles
            .clone()
            .filter_map(|role| match run.outcomes.get(&(check.name, role)) {
                Some(Ok(())) => None,
                Some(Err(reason)) => Some(format!("{role}: {reason}")),
                None => Some(format!("{role}: timed out")),
            })
            .collect();

        lines.push(if failures.is_empty() {
            format!("PASS {}", check.name)
        } else {
            format!("FAIL {}: {}", check.name, failures.join("; "))
        });
    }

    lines.push("DONE".to_string());

    if let Ok(path) = std::env::var(RESULTS) {
        if let Err(error) = std::fs::write(&path, lines.join("\n") + "\n") {
            log::error!("[RustealNetTests] cannot write {path}: {error}");
        }
    }

    for line in &lines {
        log::info!("[RustealNetTests] {line}");
    }
}

#[uclass(parent = RustealWorldSubsystem)]
pub struct NetTestDirector {
    role: Option<Role>,
}

#[uclass_impl]
impl NetTestDirector {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        self.as_ref().checked()?.set_wants_tick(true);
        Ok(())
    }

    #[ufunction(Override)]
    fn receive_should_create_subsystem(&self, _outer: UObjectRef<Object>) -> bool {
        std::env::var_os(RESULTS).is_some()
    }

    #[ufunction(Override)]
    fn receive_world_begin_play(&mut self) {
        let world = self.as_ref().upcast_to::<Object>();

        let Ok(mut guard) = RUN.lock() else {
            return;
        };
        let run = guard.get_or_insert_with(NetRun::default);

        if KismetSystemLibrary::is_standalone(world) {
            run.outcomes.insert(
                ("net_play_in_editor", Role::Server),
                Err("Play In Editor started without a server".to_string()),
            );
            finish(run);
            return;
        }

        run.started.get_or_insert_with(Instant::now);

        self.set_role(Some(if KismetSystemLibrary::is_dedicated_server(world) {
            Role::Server
        } else {
            run.clients += 1;
            Role::Client(run.clients - 1)
        }));
    }

    #[ufunction(Override)]
    fn receive_tick(&mut self, _delta_seconds: f32) {
        let Some(role) = self.role() else {
            return;
        };
        let world = self.as_ref().upcast_to::<Object>();

        let Ok(mut guard) = RUN.lock() else {
            return;
        };
        let Some(run) = guard.as_mut() else {
            return;
        };

        if run.finished {
            return;
        }

        for check in checks() {
            if run.outcomes.contains_key(&(check.name, role)) {
                continue;
            }

            if let Some(outcome) = (check.check)(role, &world) {
                run.outcomes.insert((check.name, role), outcome);
            }
        }

        let expected = checks().len() * (CLIENTS + 1);
        let timed_out = run.started.is_some_and(|started| started.elapsed() > TIMEOUT);

        if run.outcomes.len() == expected || timed_out {
            finish(run);
        }
    }
}
