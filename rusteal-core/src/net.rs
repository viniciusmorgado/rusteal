use crate::error::{RustealResult, check_ffi};
use crate::ffi_dispatch;
use crate::object_ref::UObjectRef;
use crate::traits::UeClass;

/// Registers `object`, an object of a class deriving from `RustealReplicatedObject` whose outer
/// is `actor`, as one of `actor`'s replicated subobjects: its replicated properties reach the
/// clients `actor` replicates to, with `actor`'s connection for its remote functions. Call it on
/// the server; `condition` is a replication condition's value (`0` for none).
#[track_caller]
pub fn add_replicated_subobject(
    actor: &UObjectRef<impl UeClass>,
    object: &UObjectRef<impl UeClass>,
    condition: u8,
) -> RustealResult<()> {
    check_ffi(unsafe {
        ffi_dispatch::world_add_replicated_subobject(
            actor.checked()?.raw(),
            object.checked()?.raw(),
            condition,
        )
    })
}

/// Stops replicating `object` with `actor`.
#[track_caller]
pub fn remove_replicated_subobject(
    actor: &UObjectRef<impl UeClass>,
    object: &UObjectRef<impl UeClass>,
) -> RustealResult<()> {
    check_ffi(unsafe {
        ffi_dispatch::world_remove_replicated_subobject(actor.checked()?.raw(), object.raw())
    })
}
