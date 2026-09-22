// Type-safe UMG widget helpers on top of rusteal_core::widget raw functions.

use rusteal_core::{UObjectRef, UeClass, RustealResult};

/// Create a UMG widget of type `T` (must be a UUserWidget subclass).
///
/// `owner` should be a PlayerController, World, or GameInstance.
pub fn create_widget<T: UeClass>(
    owner: &UObjectRef<impl UeClass>,
) -> RustealResult<UObjectRef<T>> {
    let owner_handle = owner.checked()?.raw();
    let class = T::static_class();
    let handle = rusteal_core::widget::create_widget_raw(owner_handle, class)?;
    Ok(unsafe { UObjectRef::from_raw(handle) })
}

/// Create a child widget of type `T`, using the given UUserWidget's WidgetTree as outer.
///
/// This is useful for programmatic widget tree construction where child widgets
/// need to be owned by the parent widget's WidgetTree.
pub fn create_child_widget<T: UeClass>(
    parent_user_widget: &UObjectRef<impl UeClass>,
) -> RustealResult<UObjectRef<T>> {
    let parent_handle = parent_user_widget.checked()?.raw();
    let tree_handle = rusteal_core::widget::get_widget_tree_raw(parent_handle)?;
    let class = T::static_class();
    let handle = rusteal_core::world::new_object_raw(tree_handle, class)?;
    Ok(unsafe { UObjectRef::from_raw(handle) })
}

/// Set the root widget of a UUserWidget's WidgetTree.
pub fn set_root_widget(
    user_widget: &UObjectRef<impl UeClass>,
    root_widget: &UObjectRef<impl UeClass>,
) -> RustealResult<()> {
    let uw_handle = user_widget.checked()?.raw();
    let rw_handle = root_widget.checked()?.raw();
    rusteal_core::widget::set_root_widget_raw(uw_handle, rw_handle)
}

/// Get the WidgetTree from a UUserWidget.
pub fn get_widget_tree(
    user_widget: &UObjectRef<impl UeClass>,
) -> RustealResult<rusteal_ffi::UObjectHandle> {
    let handle = user_widget.checked()?.raw();
    rusteal_core::widget::get_widget_tree_raw(handle)
}
