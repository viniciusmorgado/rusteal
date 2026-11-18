//! The game thread, as a value: what engine objects borrow from when nothing else hands them out.

use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use crate::error::{RustealError, RustealResult};

/// Proof that the caller is on the game thread, where engine objects live. Functions that hand
/// out an object without taking one (finding or loading an object by path, making one with no
/// outer, engine functions with no object parameter) borrow it, so what they return lives no
/// longer than this value: `let gt = GameThread::current()?;`.
///
/// It is not `Send`: it stays on the thread it proves.
#[derive(Debug)]
pub struct GameThread(PhantomData<*const ()>);

impl GameThread {
    /// The game thread, when the caller is on it.
    #[track_caller]
    pub fn current() -> RustealResult<Self> {
        if crate::sync::on_game_thread() {
            Ok(GameThread(PhantomData))
        } else {
            Err(RustealError::invalid_operation(
                "GameThread::current: not on the game thread",
            ))
        }
    }
}

/// A Rust class's object (the `self` of its `#[ufunction]`s), for as long as the reference it was
/// found from may live: [`from_obj`](crate::RustObject) hands one out, so the class's Rust data
/// cannot be reached once the call that found it is over.
pub struct RustRef<'a, T> {
    value: T,
    _scope: PhantomData<&'a ()>,
}

impl<T> RustRef<'_, T> {
    #[doc(hidden)]
    pub fn new(value: T) -> Self {
        RustRef {
            value,
            _scope: PhantomData,
        }
    }
}

impl<'a, T> RustRef<'a, T> {
    /// The class turned into something else for the same call, such as a trait object:
    /// `CombatEnemy::from_obj(actor)?.map(|e| Box::new(e) as Box<dyn Damageable>)`.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> RustRef<'a, U> {
        RustRef {
            value: f(self.value),
            _scope: PhantomData,
        }
    }
}

impl<T> Deref for RustRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> DerefMut for RustRef<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for RustRef<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}
