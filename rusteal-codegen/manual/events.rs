use rusteal_core::UObjectRef;
use rusteal_core::events::NativeEvent;
use rusteal_ffi::{RustealNativeEvent, RustealNativeEventArgs, UObjectHandle};

use crate::engine::{Controller, GameModeBase, Level, PlayerController, World};

fn first<T: rusteal_core::UeClass>(args: &RustealNativeEventArgs) -> UObjectRef<T> {
    unsafe { UObjectRef::from_raw(args.first) }
}

fn second<T: rusteal_core::UeClass>(args: &RustealNativeEventArgs) -> UObjectRef<T> {
    unsafe { UObjectRef::from_raw(args.second) }
}

fn global<A: Send + 'static>(
    kind: RustealNativeEvent,
    convert: fn(&RustealNativeEventArgs) -> A,
) -> NativeEvent<A> {
    NativeEvent::new(kind, UObjectHandle::null(), convert)
}

/// The application's lifecycle, `FCoreDelegates`.
pub mod core_events {
    use super::*;

    /// Before the engine exits.
    pub fn pre_exit() -> NativeEvent<()> {
        global(RustealNativeEvent::PreExit, |_| ())
    }

    /// The application is about to go to the background (a phone's home button).
    pub fn will_enter_background() -> NativeEvent<()> {
        global(RustealNativeEvent::WillEnterBackground, |_| ())
    }

    /// The application came back to the foreground.
    pub fn has_entered_foreground() -> NativeEvent<()> {
        global(RustealNativeEvent::HasEnteredForeground, |_| ())
    }

    /// The application is about to lose the focus.
    pub fn will_deactivate() -> NativeEvent<()> {
        global(RustealNativeEvent::WillDeactivate, |_| ())
    }

    /// The application got the focus back.
    pub fn has_reactivated() -> NativeEvent<()> {
        global(RustealNativeEvent::HasReactivated, |_| ())
    }
}

/// Worlds and their levels, `FWorldDelegates`.
pub mod world_events {
    use super::*;

    /// A world was initialized.
    pub fn initialized() -> NativeEvent<UObjectRef<World>> {
        global(RustealNativeEvent::WorldInitialized, first)
    }

    /// A world is cleaned up, with whether its session ended.
    pub fn cleanup() -> NativeEvent<(UObjectRef<World>, bool)> {
        global(RustealNativeEvent::WorldCleanup, |args| (first(args), args.a != 0))
    }

    /// A level was added to a world (a streamed level became visible).
    pub fn level_added() -> NativeEvent<(UObjectRef<Level>, UObjectRef<World>)> {
        global(RustealNativeEvent::LevelAdded, |args| (first(args), second(args)))
    }

    /// A level was removed from a world.
    pub fn level_removed() -> NativeEvent<(UObjectRef<Level>, UObjectRef<World>)> {
        global(RustealNativeEvent::LevelRemoved, |args| (first(args), second(args)))
    }
}

/// Players joining and leaving, `FGameModeEvents`.
pub mod game_mode_events {
    use super::*;

    /// A player logged in, after the game mode's own PostLogin.
    pub fn post_login() -> NativeEvent<(UObjectRef<GameModeBase>, UObjectRef<PlayerController>)> {
        global(RustealNativeEvent::PostLogin, |args| (first(args), second(args)))
    }

    /// A player is logging out.
    pub fn logout() -> NativeEvent<(UObjectRef<GameModeBase>, UObjectRef<Controller>)> {
        global(RustealNativeEvent::Logout, |args| (first(args), second(args)))
    }
}

#[cfg(feature = "ai")]
pub use path_following::PathFollowingEventsExt;

#[cfg(feature = "ai")]
mod path_following {
    use super::*;
    use crate::ai::{EPathFollowingResult, PathFollowingComponent};
    use rusteal_core::UeEnum;

    /// A move of a path following component finished: its request id and result.
    pub trait PathFollowingEventsExt {
        fn on_request_finished(&self) -> NativeEvent<(u32, EPathFollowingResult)>;
    }

    impl PathFollowingEventsExt for UObjectRef<PathFollowingComponent> {
        fn on_request_finished(&self) -> NativeEvent<(u32, EPathFollowingResult)> {
            NativeEvent::new(RustealNativeEvent::MoveRequestFinished, self.raw(), |args| {
                let code = EPathFollowingResult::from_i64(i64::from(args.b & 0xff))
                    .unwrap_or(EPathFollowingResult::Invalid);
                (args.a, code)
            })
        }
    }
}
