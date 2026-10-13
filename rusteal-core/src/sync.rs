pub(crate) use std::sync::{
    Mutex, MutexGuard, Once, OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
pub(crate) use std::thread_local;

pub(crate) fn lock_or_recover<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub(crate) fn read_or_recover<T>(m: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    m.read().unwrap_or_else(|e| e.into_inner())
}

pub(crate) fn write_or_recover<T>(m: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    m.write().unwrap_or_else(|e| e.into_inner())
}
