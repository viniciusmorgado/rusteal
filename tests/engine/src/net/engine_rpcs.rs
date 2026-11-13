use std::collections::HashSet;
use std::sync::Mutex;

use bindings::engine::{ControllerExt, GameplayStatics, PlayerControllerExt, PlayerStateExt};

use super::{CLIENTS, NetWorld, Role};

const PREFIX: &str = "RustealClient";

static RENAMED: Mutex<Option<HashSet<usize>>> = Mutex::new(None);

fn player_name(net: &NetWorld, index: i32) -> Option<String> {
    let controller = GameplayStatics::get_player_controller(net.context, index)
        .checked()
        .ok()?;
    let state = controller.get_player_state().checked().ok()?;
    Some(state.get_player_name())
}

pub fn renamed(net: &NetWorld) -> bool {
    player_name(net, 0).is_some_and(|name| name.starts_with(PREFIX))
}

pub fn engine_server_function_runs_on_the_server(net: &NetWorld) -> Option<Result<(), String>> {
    match net.role {
        Role::Server => {
            let names: HashSet<String> = (0..CLIENTS as i32)
                .filter_map(|index| player_name(net, index))
                .filter(|name| name.starts_with(PREFIX))
                .collect();

            (names.len() == CLIENTS).then_some(Ok(()))
        }
        Role::Client(index) => {
            let wanted = format!("{PREFIX}{index}");
            let controller = GameplayStatics::get_player_controller(net.context, 0)
                .checked()
                .ok()?;
            let key = controller.raw().0 as usize;

            let first = RENAMED
                .lock()
                .ok()?
                .get_or_insert_with(HashSet::new)
                .insert(key);

            if first {
                controller.server_change_name(&wanted);
            }

            (player_name(net, 0)? == wanted).then_some(Ok(()))
        }
    }
}
