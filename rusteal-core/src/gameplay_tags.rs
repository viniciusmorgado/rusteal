use crate::error::{RustealResult, check_ffi};
use crate::ffi_dispatch;

/// Adds `name` to the project's gameplay tags, with its parents (`Combat.Damage.Fire` adds
/// `Combat` and `Combat.Damage` too), as a native tag of the engine; declaring it again does
/// nothing. The bindings' `declare_gameplay_tag` gives it as an `FGameplayTag`.
#[track_caller]
pub fn declare(name: &str, comment: &str) -> RustealResult<()> {
    check_ffi(unsafe {
        ffi_dispatch::world_add_gameplay_tag(
            name.as_ptr(),
            name.len() as u32,
            comment.as_ptr(),
            comment.len() as u32,
        )
    })
}

/// Whether `name` is a gameplay tag of the project: declared in Rust, in C++, or in its
/// settings.
pub fn is_registered(name: &str) -> bool {
    unsafe { ffi_dispatch::world_gameplay_tag_is_registered(name.as_ptr(), name.len() as u32) }
}
