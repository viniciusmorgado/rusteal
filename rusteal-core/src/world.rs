use rusteal_ffi::{FNameHandle, UClassHandle, UObjectHandle, UStructHandle};

use crate::error::{RustealError, RustealResult, check_ffi};
use crate::ffi_dispatch;

pub fn spawn_actor_raw(
    world: UObjectHandle,
    class: UClassHandle,
    transform_buf: &[u8],
    owner: UObjectHandle,
) -> RustealResult<UObjectHandle> {
    let result = unsafe {
        ffi_dispatch::world_spawn_actor(
            world,
            class,
            transform_buf.as_ptr(),
            transform_buf.len() as u32,
            owner,
        )
    };

    if result.is_null() {
        Err(RustealError::InvalidOperation(
            "spawn_actor returned null".into(),
        ))
    } else {
        Ok(result)
    }
}

pub fn find_object_raw(class: UClassHandle, path: &str) -> RustealResult<UObjectHandle> {
    let result =
        unsafe { ffi_dispatch::world_find_object(class, path.as_ptr(), path.len() as u32) };

    if result.is_null() {
        Err(RustealError::InvalidOperation(format!(
            "find_object: not found: {path}"
        )))
    } else {
        Ok(result)
    }
}

pub fn load_object_raw(class: UClassHandle, path: &str) -> RustealResult<UObjectHandle> {
    let result =
        unsafe { ffi_dispatch::world_load_object(class, path.as_ptr(), path.len() as u32) };

    if result.is_null() {
        Err(RustealError::InvalidOperation(format!(
            "load_object: failed to load: {path}"
        )))
    } else {
        Ok(result)
    }
}

pub fn new_object_raw(outer: UObjectHandle, class: UClassHandle) -> RustealResult<UObjectHandle> {
    let result = unsafe { ffi_dispatch::world_new_object(outer, class) };

    if result.is_null() {
        Err(RustealError::InvalidOperation(
            "new_object returned null".into(),
        ))
    } else {
        Ok(result)
    }
}

pub fn spawn_actor_deferred_raw(
    world: UObjectHandle,
    class: UClassHandle,
    transform_buf: &[u8],
    owner: UObjectHandle,
    instigator: UObjectHandle,
    collision_method: u8,
) -> RustealResult<UObjectHandle> {
    let result = unsafe {
        ffi_dispatch::world_spawn_actor_deferred(
            world,
            class,
            transform_buf.as_ptr(),
            transform_buf.len() as u32,
            owner,
            instigator,
            collision_method,
        )
    };

    if result.is_null() {
        Err(RustealError::InvalidOperation(
            "spawn_actor_deferred returned null".into(),
        ))
    } else {
        Ok(result)
    }
}

pub fn finish_spawning_raw(actor: UObjectHandle, transform_buf: &[u8]) -> RustealResult<()> {
    check_ffi(unsafe {
        ffi_dispatch::world_finish_spawning(
            actor,
            transform_buf.as_ptr(),
            transform_buf.len() as u32,
        )
    })
}

pub fn get_world_raw(object: UObjectHandle) -> RustealResult<UObjectHandle> {
    let result = unsafe { ffi_dispatch::world_get_world(object) };

    if result.is_null() {
        Err(RustealError::InvalidOperation(
            "get_world returned null".into(),
        ))
    } else {
        Ok(result)
    }
}

pub fn get_all_actors_of_class_raw(
    world: UObjectHandle,
    class: UClassHandle,
) -> RustealResult<Vec<UObjectHandle>> {
    let handle_size = core::mem::size_of::<UObjectHandle>() as u32;

    let mut count: u32 = 0;

    check_ffi(unsafe {
        ffi_dispatch::world_get_all_actors_of_class(
            world,
            class,
            std::ptr::null_mut(),
            0,
            &mut count,
        )
    })?;

    if count == 0 {
        return Ok(Vec::new());
    }

    let byte_size = count * handle_size;
    let mut buf = vec![0u8; byte_size as usize];
    let mut actual_count: u32 = 0;

    check_ffi(unsafe {
        ffi_dispatch::world_get_all_actors_of_class(
            world,
            class,
            buf.as_mut_ptr(),
            byte_size,
            &mut actual_count,
        )
    })?;

    let handles = buf
        .chunks_exact(handle_size as usize)
        .take(actual_count as usize)
        .map(|chunk| {
            let bytes: [u8; 8] = chunk.try_into().expect("handle is 8 bytes");
            let ptr = usize::from_ne_bytes(bytes) as *mut std::ffi::c_void;

            UObjectHandle(ptr)
        })
        .collect();

    Ok(handles)
}

pub fn find_data_table_row_raw(
    table: UObjectHandle,
    row_name: FNameHandle,
    row_struct: UStructHandle,
) -> Option<*mut u8> {
    let row = unsafe { ffi_dispatch::world_find_data_table_row(table, row_name, row_struct) };
    (!row.is_null()).then_some(row)
}
