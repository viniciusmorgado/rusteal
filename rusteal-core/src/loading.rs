use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use rusteal_ffi::UObjectHandle;

use crate::delegate_registry::{register_callback, unregister_callback};
use crate::error::{RustealError, RustealResult, check_ffi};
use crate::ffi_dispatch;
use crate::lock_or_recover;
use crate::object_ref::UObjectRef;
use crate::soft_ref::SoftObjectRef;
use crate::sync::Mutex;
use crate::traits::UeClass;

#[derive(Default)]
struct State {
    done: bool,
    waker: Option<Waker>,
}

struct Finish(Arc<Mutex<State>>);

impl Finish {
    fn finish(&self) {
        let waker = {
            let mut state = lock_or_recover(&self.0);
            state.done = true;
            state.waker.take()
        };

        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl Drop for Finish {
    fn drop(&mut self) {
        self.finish();
    }
}

struct Request {
    id: u64,
    count: usize,
    state: Arc<Mutex<State>>,
    finished: bool,
}

impl Request {
    fn new(count: usize) -> Self {
        let state = Arc::new(Mutex::new(State::default()));
        let finish = Finish(state.clone());
        let id = register_callback(move |_| finish.finish());

        Request {
            id,
            count,
            state,
            finished: false,
        }
    }

    fn objects(&mut self) -> RustealResult<Vec<UObjectHandle>> {
        self.finished = true;
        unregister_callback(self.id);

        let mut objects = vec![UObjectHandle::null(); self.count];
        let mut count = 0u32;

        check_ffi(unsafe {
            ffi_dispatch::world_load_finish(
                self.id,
                objects.as_mut_ptr(),
                objects.len() as u32,
                &mut count,
            )
        })?;

        objects.truncate(count as usize);
        Ok(objects)
    }
}

impl Future for Request {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut state = lock_or_recover(&self.state);

        if state.done {
            return Poll::Ready(());
        }

        state.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

impl Drop for Request {
    fn drop(&mut self) {
        if self.finished {
            return;
        }

        if crate::api::is_api_initialized() {
            let _ = unsafe { ffi_dispatch::world_load_cancel(self.id) };
        }

        unregister_callback(self.id);
    }
}

async fn load_paths(paths: Vec<String>) -> RustealResult<Vec<UObjectHandle>> {
    let joined = paths.join("\n");
    let mut request = Request::new(paths.len());

    check_ffi(unsafe {
        ffi_dispatch::world_load_async(joined.as_ptr(), joined.len() as u32, request.id)
    })?;

    (&mut request).await;
    request.objects()
}

fn typed<T: UeClass>(handle: UObjectHandle, path: &str) -> RustealResult<UObjectRef<T>> {
    if handle.is_null() {
        return Err(RustealError::invalid_operation(format!(
            "load_async: failed to load: {path}"
        )));
    }

    if !unsafe { ffi_dispatch::core_is_a(handle, T::static_class()) } {
        return Err(RustealError::invalid_cast_to(handle, T::static_class())
            .context(format!("load_async: {path}")));
    }

    Ok(unsafe { UObjectRef::from_raw(handle) })
}

pub(crate) async fn load_raw(path: String) -> RustealResult<UObjectHandle> {
    let handle = load_paths(vec![path.clone()])
        .await?
        .first()
        .copied()
        .unwrap_or(UObjectHandle::null());

    if handle.is_null() {
        return Err(RustealError::invalid_operation(format!(
            "load_async: failed to load: {path}"
        )));
    }

    Ok(handle)
}

/// Loads `asset` in the background, resuming when it is in memory.
///
/// The loaded object is only referenced by what keeps it: store it in a `#[uproperty]`, or pin it,
/// before the next garbage collection.
pub(crate) async fn load_one<T: UeClass>(path: String) -> RustealResult<UObjectRef<T>> {
    let handle = load_paths(vec![path.clone()])
        .await?
        .first()
        .copied()
        .unwrap_or(UObjectHandle::null());

    typed(handle, &path)
}

/// Loads every asset of `assets` in one request, resuming when all are in memory; each entry is
/// that asset or why it did not load.
pub async fn load_all<T: UeClass>(
    assets: &[SoftObjectRef<T>],
) -> RustealResult<Vec<RustealResult<UObjectRef<T>>>> {
    let paths: Vec<String> = assets.iter().map(|a| a.path().to_string()).collect();
    let handles = load_paths(paths.clone()).await?;

    Ok(paths
        .iter()
        .enumerate()
        .map(|(i, path)| {
            typed(
                handles.get(i).copied().unwrap_or(UObjectHandle::null()),
                path,
            )
        })
        .collect())
}

/// Loads the primary asset `id` (as `"Type:Name"`) with `bundles`, through the asset manager.
pub async fn load_primary_asset<T: UeClass>(
    id: &str,
    bundles: &[&str],
) -> RustealResult<UObjectRef<T>> {
    let bundles = bundles.join("\n");
    let mut request = Request::new(1);

    check_ffi(unsafe {
        ffi_dispatch::world_load_primary_assets(
            id.as_ptr(),
            id.len() as u32,
            bundles.as_ptr(),
            bundles.len() as u32,
            request.id,
        )
    })?;

    (&mut request).await;

    let handle = request
        .objects()?
        .first()
        .copied()
        .unwrap_or(UObjectHandle::null());

    typed(handle, id)
}

#[doc(hidden)]
pub fn loads_in_progress() -> u32 {
    unsafe { ffi_dispatch::world_loads_in_progress() }
}
