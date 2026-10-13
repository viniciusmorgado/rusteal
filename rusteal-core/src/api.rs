use crate::sync::OnceLock;

use rusteal_ffi::RustealApiTable;

struct ApiRef(*const RustealApiTable);
unsafe impl Send for ApiRef {}
unsafe impl Sync for ApiRef {}

static API: OnceLock<ApiRef> = OnceLock::new();

pub fn init_api(table: *const RustealApiTable) {
    assert!(!table.is_null(), "init_api called with null pointer");

    let _ = API.set(ApiRef(table));
}

#[inline(always)]
pub fn api() -> &'static RustealApiTable {
    unsafe { &*API.get().expect("rusteal API not initialized").0 }
}

#[inline]
pub fn is_api_initialized() -> bool {
    API.get().is_some()
}
