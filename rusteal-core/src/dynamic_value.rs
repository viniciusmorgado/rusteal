use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use rusteal_ffi::{FPropertyHandle, UClassHandle, UObjectHandle};

use crate::containers::{ContainerElement, OwnedStruct, UeArray, UeMap, UeSet};
use crate::error::{RustealResult, check_ffi};
use crate::ffi_dispatch;
use crate::fname::FName;
use crate::object_ref::{ObjectPointer, UObjectRef};
use crate::script_interface::ScriptInterface;
use crate::soft_class_ref::SoftClassRef;
use crate::soft_ref::SoftObjectRef;
use crate::strings::take_utf8;
use crate::subclass_of::SubclassOf;
use crate::text::Text;
use crate::traits::{UeClass, UeInterface, UeStruct};
use crate::weak_ptr::TWeakObjectPtr;

/// A value a reflected property holds, read from and written to any container of it:
/// an object, a struct, or a function's parameters.
pub trait DynamicValue: Sized {
    fn write_to(&self, container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<()>;

    fn read_from(container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<Self>;
}

macro_rules! raw_value {
    ($($ty:ty),*) => {
        $(
            impl DynamicValue for $ty {
                fn write_to(&self, container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<()> {
                    check_ffi(unsafe {
                        ffi_dispatch::property_set_property_at(
                            container,
                            prop,
                            0,
                            self as *const $ty as *const u8,
                            std::mem::size_of::<$ty>() as u32,
                        )
                    })
                }

                fn read_from(container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<Self> {
                    let mut value = <$ty>::default();

                    check_ffi(unsafe {
                        ffi_dispatch::property_get_property_at(
                            container,
                            prop,
                            0,
                            &mut value as *mut $ty as *mut u8,
                            std::mem::size_of::<$ty>() as u32,
                        )
                    })?;

                    Ok(value)
                }
            }
        )*
    };
}

raw_value!(bool, i8, i16, i32, i64, u8, u16, u32, u64, f32, f64);

fn read_path(container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<String> {
    let mut buf = vec![0u8; 256];
    let mut len: u32 = 0;

    check_ffi(unsafe {
        ffi_dispatch::property_get_soft_object_path(
            container,
            prop,
            buf.as_mut_ptr(),
            buf.len() as u32,
            &mut len,
        )
    })?;

    if len as usize > buf.len() {
        buf = vec![0u8; len as usize];

        check_ffi(unsafe {
            ffi_dispatch::property_get_soft_object_path(
                container,
                prop,
                buf.as_mut_ptr(),
                buf.len() as u32,
                &mut len,
            )
        })?;
    }

    buf.truncate(len as usize);
    Ok(String::from_utf8(buf).unwrap_or_default())
}

fn write_path(container: UObjectHandle, prop: FPropertyHandle, path: &str) -> RustealResult<()> {
    check_ffi(unsafe {
        ffi_dispatch::property_set_soft_object_path(
            container,
            prop,
            path.as_ptr(),
            path.len() as u32,
        )
    })
}

fn read_object(container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<UObjectHandle> {
    let mut handle = UObjectHandle::null();
    check_ffi(unsafe { ffi_dispatch::property_get_object(container, prop, &mut handle) })?;

    Ok(handle)
}

fn write_object(
    container: UObjectHandle,
    prop: FPropertyHandle,
    handle: UObjectHandle,
) -> RustealResult<()> {
    check_ffi(unsafe { ffi_dispatch::property_set_object(container, prop, handle) })
}

impl DynamicValue for String {
    fn write_to(&self, container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<()> {
        check_ffi(unsafe {
            ffi_dispatch::property_set_string(container, prop, self.as_ptr(), self.len() as u32)
        })
    }

    fn read_from(container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<Self> {
        let mut buf = vec![0u8; 256];
        let mut len: u32 = 0;

        check_ffi(unsafe {
            ffi_dispatch::property_get_string(
                container,
                prop,
                buf.as_mut_ptr(),
                buf.len() as u32,
                &mut len,
            )
        })?;

        Ok(take_utf8(buf, len))
    }
}

impl DynamicValue for FName {
    fn write_to(&self, container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<()> {
        check_ffi(unsafe { ffi_dispatch::property_set_fname(container, prop, self.handle()) })
    }

    fn read_from(container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<Self> {
        let mut handle = FName::NONE.handle();
        check_ffi(unsafe { ffi_dispatch::property_get_fname(container, prop, &mut handle) })?;

        Ok(FName(handle))
    }
}

impl DynamicValue for Text {
    fn write_to(&self, container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<()> {
        check_ffi(unsafe { ffi_dispatch::property_set_text(container, prop, self.as_raw()) })
    }

    fn read_from(container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<Self> {
        let mut raw: *mut u8 = std::ptr::null_mut();
        check_ffi(unsafe { ffi_dispatch::property_get_text(container, prop, &mut raw) })?;

        Ok(unsafe { Text::from_raw(raw) })
    }
}

impl<T: UeStruct> DynamicValue for OwnedStruct<T> {
    fn write_to(&self, container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<()> {
        let bytes = self.as_bytes();

        check_ffi(unsafe {
            ffi_dispatch::property_set_struct(container, prop, bytes.as_ptr(), bytes.len() as u32)
        })
    }

    fn read_from(container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<Self> {
        let size = unsafe { ffi_dispatch::reflection_get_property_size(prop) } as usize;
        let mut buf = vec![0u8; size];

        check_ffi(unsafe {
            ffi_dispatch::property_get_struct(container, prop, buf.as_mut_ptr(), size as u32)
        })?;

        Ok(unsafe { OwnedStruct::from_constructed(buf) })
    }
}

impl<T: UeClass> DynamicValue for UObjectRef<T> {
    fn write_to(&self, container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<()> {
        write_object(container, prop, self.raw())
    }

    fn read_from(container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<Self> {
        Ok(unsafe { UObjectRef::from_raw(read_object(container, prop)?) })
    }
}

impl<T: UeClass> DynamicValue for SubclassOf<T> {
    fn write_to(&self, container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<()> {
        write_object(container, prop, UObjectHandle(self.raw().0))
    }

    fn read_from(container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<Self> {
        Ok(unsafe { SubclassOf::from_raw(UClassHandle(read_object(container, prop)?.0)) })
    }
}

impl<T: UeClass> DynamicValue for TWeakObjectPtr<T> {
    fn write_to(&self, container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<()> {
        write_object(container, prop, self.object_handle())
    }

    fn read_from(container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<Self> {
        Ok(unsafe { TWeakObjectPtr::from_object_handle(read_object(container, prop)?) })
    }
}

impl<I: UeInterface + ?Sized> DynamicValue for Option<ScriptInterface<I>> {
    fn write_to(&self, container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<()> {
        write_object(
            container,
            prop,
            self.map_or(UObjectHandle::null(), |i| i.raw()),
        )
    }

    fn read_from(container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<Self> {
        Ok(ScriptInterface::new(read_object(container, prop)?).ok())
    }
}

impl<T: UeClass> DynamicValue for SoftObjectRef<T> {
    fn write_to(&self, container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<()> {
        write_path(container, prop, self.path())
    }

    fn read_from(container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<Self> {
        Ok(SoftObjectRef::new(read_path(container, prop)?))
    }
}

impl<T: UeClass> DynamicValue for SoftClassRef<T> {
    fn write_to(&self, container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<()> {
        write_path(container, prop, self.path())
    }

    fn read_from(container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<Self> {
        Ok(SoftClassRef::new(read_path(container, prop)?))
    }
}

impl<T: ContainerElement> DynamicValue for Vec<T> {
    fn write_to(&self, container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<()> {
        UeArray::<T>::new(container, prop).set_all(self)
    }

    fn read_from(container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<Self> {
        UeArray::<T>::new(container, prop).to_vec()
    }
}

impl<T: ContainerElement + Hash + Eq> DynamicValue for HashSet<T> {
    fn write_to(&self, container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<()> {
        let set = UeSet::<T>::new(container, prop);
        set.clear()?;

        for item in self {
            set.add(item)?;
        }

        Ok(())
    }

    fn read_from(container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<Self> {
        UeSet::<T>::new(container, prop).to_hash_set()
    }
}

impl<K: ContainerElement + Hash + Eq, V: ContainerElement> DynamicValue for HashMap<K, V> {
    fn write_to(&self, container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<()> {
        let map = UeMap::<K, V>::new(container, prop);
        map.clear()?;

        for (key, value) in self {
            map.add(key, value)?;
        }

        Ok(())
    }

    fn read_from(container: UObjectHandle, prop: FPropertyHandle) -> RustealResult<Self> {
        UeMap::<K, V>::new(container, prop).to_hash_map()
    }
}
