use rusteal_ffi::{FPropertyHandle, UClassHandle, UObjectHandle};

use crate::dynamic_value::DynamicValue;
use crate::error::{RustealError, RustealResult};
use crate::ffi_dispatch;
use crate::lock_or_recover;
use crate::sync::Mutex;

/// The class at `path` (`/Script/<Library>.<Class>`), looked up each time: a library's hot reload
/// replaces its classes, so a cached handle could name a class that is gone.
#[doc(hidden)]
pub fn class_at(path: &str) -> UClassHandle {
    let class_class = unsafe { ffi_dispatch::reflection_get_static_class(b"Class\0".as_ptr(), 5) };
    let handle =
        unsafe { ffi_dispatch::world_find_object(class_class, path.as_ptr(), path.len() as u32) };

    UClassHandle(handle.0)
}

/// A property of a class from another library, found by name and kept until that class is
/// replaced.
#[doc(hidden)]
pub struct PropertySlot {
    name: &'static str,
    found: Mutex<Option<(UClassHandle, FPropertyHandle)>>,
}

impl PropertySlot {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            found: Mutex::new(None),
        }
    }

    fn property(&self, class: UClassHandle) -> RustealResult<FPropertyHandle> {
        let mut found = lock_or_recover(&self.found);

        if let Some((cached, prop)) = *found
            && cached == class
        {
            return Ok(prop);
        }

        if class.is_null() {
            return Err(RustealError::invalid_operation(format!(
                "the class of property {} is not loaded",
                self.name
            )));
        }

        let prop = unsafe {
            ffi_dispatch::reflection_find_property(
                class,
                self.name.as_ptr(),
                self.name.len() as u32,
            )
        };

        if prop.is_null() {
            return Err(RustealError::property_not_found(self.name));
        }

        *found = Some((class, prop));
        Ok(prop)
    }

    #[track_caller]
    pub fn get<T: DynamicValue + Default>(&self, object: UObjectHandle, class: UClassHandle) -> T {
        match self
            .property(class)
            .and_then(|prop| T::read_from(object, prop))
        {
            Ok(value) => value,
            Err(e) => {
                e.__log_in(crate::LOG_WARNING, &format!("reading {}", self.name));
                T::default()
            }
        }
    }

    #[track_caller]
    pub fn set<T: DynamicValue>(&self, object: UObjectHandle, class: UClassHandle, value: &T) {
        if let Err(e) = self
            .property(class)
            .and_then(|prop| value.write_to(object, prop))
        {
            e.__log_in(crate::LOG_WARNING, &format!("writing {}", self.name));
        }
    }

    /// [`PropertySlot::get`] for an enum, copied as the bytes the property holds.
    #[track_caller]
    pub fn get_copy<T: Copy>(&self, object: UObjectHandle, class: UClassHandle) -> Option<T> {
        let prop = match self.property(class) {
            Ok(prop) => prop,
            Err(e) => {
                e.__log_in(crate::LOG_WARNING, &format!("reading {}", self.name));
                return None;
            }
        };

        let mut value = std::mem::MaybeUninit::<T>::zeroed();

        let read = crate::error::check_ffi(unsafe {
            ffi_dispatch::property_get_property_at(
                object,
                prop,
                0,
                value.as_mut_ptr() as *mut u8,
                std::mem::size_of::<T>() as u32,
            )
        });

        match read {
            Ok(()) => Some(unsafe { value.assume_init() }),
            Err(e) => {
                e.__log_in(crate::LOG_WARNING, &format!("reading {}", self.name));
                None
            }
        }
    }

    /// [`PropertySlot::set`] for an enum.
    #[track_caller]
    pub fn set_copy<T: Copy>(&self, object: UObjectHandle, class: UClassHandle, value: T) {
        let written = self.property(class).and_then(|prop| {
            crate::error::check_ffi(unsafe {
                ffi_dispatch::property_set_property_at(
                    object,
                    prop,
                    0,
                    &value as *const T as *const u8,
                    std::mem::size_of::<T>() as u32,
                )
            })
        });

        if let Err(e) = written {
            e.__log_in(crate::LOG_WARNING, &format!("writing {}", self.name));
        }
    }
}

/// The class default object of `class`, which static functions are called on.
#[doc(hidden)]
pub fn default_object(class: UClassHandle) -> RustealResult<UObjectHandle> {
    if class.is_null() {
        return Err(RustealError::invalid_operation(
            "the class of a static function is not loaded".to_string(),
        ));
    }

    let cdo = unsafe { ffi_dispatch::reify_get_cdo(class) };

    if cdo.is_null() {
        Err(RustealError::object_destroyed())
    } else {
        Ok(cdo)
    }
}
