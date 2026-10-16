use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use crate::runtime::module::{self, ModuleId};
use crate::runtime::task::{self, EngineThread};
use crate::runtime::{RustealError, RustealResult};

pub use ::rayon::prelude;

pub struct ThreadPool {
    pool: Option<::rayon::ThreadPool>,
    threads: Arc<Mutex<Vec<EngineThread>>>,
}

impl ThreadPool {
    #[track_caller]
    pub fn new(threads: usize) -> RustealResult<Self> {
        let handles: Arc<Mutex<Vec<EngineThread>>> = Arc::default();
        let started = handles.clone();

        let pool = ::rayon::ThreadPoolBuilder::new()
            .num_threads(threads.max(1))
            .thread_name(|index| format!("Rusteal rayon {index}"))
            .spawn_handler(move |thread| {
                let name = thread.name().unwrap_or("Rusteal rayon").to_string();

                let handle = task::spawn_thread(&name, move || thread.run())
                    .map_err(|e| std::io::Error::other(e.to_string()))?;

                started
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push(handle);

                Ok(())
            })
            .build()
            .map_err(|e| RustealError::internal(format!("cannot start the thread pool: {e}")))?;

        Ok(Self {
            pool: Some(pool),
            threads: handles,
        })
    }

    pub fn install<R: Send>(&self, op: impl FnOnce() -> R + Send) -> R {
        self.rayon().install(op)
    }

    pub fn rayon(&self) -> &::rayon::ThreadPool {
        self.pool
            .as_ref()
            .expect("the pool lives until it is dropped")
    }

    pub fn current_num_threads(&self) -> usize {
        self.rayon().current_num_threads()
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.pool.take());

        let threads = std::mem::take(&mut *self.threads.lock().unwrap_or_else(|e| e.into_inner()));

        for thread in threads {
            thread.join();
        }
    }
}

impl std::fmt::Debug for ThreadPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThreadPool")
            .field("threads", &self.current_num_threads())
            .finish()
    }
}

static POOLS: Mutex<BTreeMap<ModuleId, Arc<ThreadPool>>> = Mutex::new(BTreeMap::new());

#[track_caller]
pub fn pool() -> RustealResult<Arc<ThreadPool>> {
    let module = module::current();
    let mut pools = POOLS.lock().unwrap_or_else(|e| e.into_inner());

    if let Some(pool) = pools.get(&module) {
        return Ok(pool.clone());
    }

    let pool = Arc::new(ThreadPool::new(task::worker_count())?);
    pools.insert(module, pool.clone());
    Ok(pool)
}

#[track_caller]
pub fn install<R: Send>(op: impl FnOnce() -> R + Send) -> RustealResult<R> {
    Ok(pool()?.install(op))
}

pub(crate) fn shutdown(module: ModuleId) {
    let pool = POOLS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&module);

    drop(pool);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::module::{FuncTable, Module};
    use prelude::*;

    static FUNCS: FuncTable = FuncTable::new(0, 0);
    static GAME: Module = Module::new("rayon_game", "", &FUNCS);

    #[test]
    fn work_runs_on_the_pool_and_the_pool_is_joined() {
        let _scope = GAME.enter();
        let sum: u64 = install(|| (1..=1000u64).into_par_iter().sum()).unwrap();
        assert_eq!(sum, 500_500);

        let same = Arc::ptr_eq(&pool().unwrap(), &pool().unwrap());
        assert!(same, "a module keeps one pool");

        let modules: Vec<ModuleId> =
            install(|| (0..64).into_par_iter().map(|_| module::current()).collect()).unwrap();

        assert!(
            modules.iter().all(|m| *m == GAME.id()),
            "the workers run in the pool's module"
        );

        shutdown(GAME.id());
        assert!(POOLS.lock().unwrap().get(&GAME.id()).is_none());
    }
}
