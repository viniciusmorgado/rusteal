use rusteal_ffi::UObjectHandle;

use crate::delegate_registry::register_callback;
use crate::error::{RustealError, RustealResult, check_ffi};
use crate::ffi_dispatch;
use crate::object_ref::UObjectRef;
use crate::slate::{IntoWidget, Widget};
use crate::traits::UeClass;

/// The details panel's layout of one class, while a customization builds it.
pub struct DetailLayout {
    layout: u64,
}

impl DetailLayout {
    /// The objects the panel shows, of the customized class.
    pub fn objects<T: UeClass>(&self) -> Vec<UObjectRef<T>> {
        let count =
            unsafe { ffi_dispatch::widget_details_objects(self.layout, std::ptr::null_mut(), 0) };
        let mut handles = vec![UObjectHandle::null(); count as usize];

        let filled = unsafe {
            ffi_dispatch::widget_details_objects(self.layout, handles.as_mut_ptr(), count)
        };

        handles.truncate(filled.min(count) as usize);

        handles
            .into_iter()
            .filter(|handle| !handle.is_null())
            .map(|handle| unsafe { UObjectRef::from_raw(handle) })
            .collect()
    }

    /// Adds a row to `category` (made when the class has none of that name): `name` on the
    /// left, `value` on the right.
    #[track_caller]
    pub fn add_row(
        &self,
        category: &str,
        name: impl IntoWidget,
        value: impl IntoWidget,
    ) -> RustealResult<()> {
        let name = name.into_widget()?.into_id();
        let value = value.into_widget()?.into_id();

        check_ffi(unsafe {
            ffi_dispatch::widget_details_add_row(
                self.layout,
                category.as_ptr(),
                category.len() as u32,
                name,
                value,
            )
        })
    }

    /// Hides the property of that Unreal name from the panel.
    #[track_caller]
    pub fn hide_property(&self, name: &str) -> RustealResult<()> {
        check_ffi(unsafe {
            ffi_dispatch::widget_details_hide_property(
                self.layout,
                name.as_ptr(),
                name.len() as u32,
            )
        })
    }
}

/// Customizes the details panel of `T` and its children, in the editor: `build` runs every time
/// a panel shows objects of the class. Registering a class again replaces its customization;
/// a library's go away when it unloads, so register them from `rusteal_runtime::on_load!`.
#[track_caller]
pub fn customize_class<T: UeClass>(
    mut build: impl FnMut(&DetailLayout) -> RustealResult<()> + Send + 'static,
) -> RustealResult<()> {
    let id = register_callback(move |params| {
        if params.is_null() {
            return;
        }

        let layout = DetailLayout {
            layout: unsafe { *(params as *const u64) },
        };

        if let Err(e) = build(&layout) {
            e.__log_in(crate::LOG_WARNING, "customizing a details panel");
        }
    });

    check_ffi(unsafe { ffi_dispatch::widget_details_customize_class(T::static_class(), id) })
}

pub fn remove_customization<T: UeClass>() {
    unsafe { ffi_dispatch::widget_details_unregister_class(T::static_class()) };
}

/// A details panel showing `object`, to place in a tab or a window built with
/// [`slate`](crate::slate); in the editor only.
#[track_caller]
pub fn view(object: &UObjectRef<impl UeClass>) -> RustealResult<Widget> {
    let handle = object.checked()?.raw();

    Widget::from_id(unsafe { ffi_dispatch::widget_details_view(handle) }).ok_or_else(|| {
        RustealError::invalid_operation(
            "details: the details panel is in the editor only".to_string(),
        )
    })
}
