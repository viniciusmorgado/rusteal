use rusteal_ffi::{UClassHandle, UObjectHandle};

use crate::error::{RustealError, RustealResult, check_ffi};
use crate::ffi_dispatch;

pub fn create_widget_raw(
    owning_object: UObjectHandle,
    widget_class: UClassHandle,
) -> RustealResult<UObjectHandle> {
    let result = unsafe { ffi_dispatch::widget_create_widget(owning_object, widget_class) };

    if result.is_null() {
        Err(RustealError::InvalidOperation(
            "create_widget returned null".into(),
        ))
    } else {
        Ok(result)
    }
}

pub fn set_root_widget_raw(
    user_widget: UObjectHandle,
    root_widget: UObjectHandle,
) -> RustealResult<()> {
    check_ffi(unsafe { ffi_dispatch::widget_set_root_widget(user_widget, root_widget) })
}

pub fn get_widget_tree_raw(user_widget: UObjectHandle) -> RustealResult<UObjectHandle> {
    let result = unsafe { ffi_dispatch::widget_get_widget_tree(user_widget) };

    if result.is_null() {
        Err(RustealError::InvalidOperation(
            "get_widget_tree returned null".into(),
        ))
    } else {
        Ok(result)
    }
}
