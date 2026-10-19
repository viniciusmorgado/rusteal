use std::fmt;
use std::marker::PhantomData;

use rusteal_ffi::UClassHandle;

use crate::error::RustealResult;
use crate::ffi_dispatch;
use crate::subclass_of::SubclassOf;
use crate::sync::OnceLock;
use crate::traits::UeClass;
use crate::world;

pub struct SoftClassRef<T: UeClass> {
    path: String,
    _marker: PhantomData<T>,
}

fn class_class() -> UClassHandle {
    static CLASS: OnceLock<usize> = OnceLock::new();

    let addr = *CLASS.get_or_init(|| {
        unsafe { ffi_dispatch::reflection_get_static_class(c"Class".as_ptr().cast(), 5) }.0 as usize
    });

    UClassHandle(addr as *mut std::ffi::c_void)
}

impl<T: UeClass> SoftClassRef<T> {
    pub fn new(path: impl Into<String>) -> Self {
        SoftClassRef {
            path: path.into(),
            _marker: PhantomData,
        }
    }

    pub fn null() -> Self {
        Self::new(String::new())
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn is_null(&self) -> bool {
        self.path.is_empty()
    }

    pub fn get(&self) -> SubclassOf<T> {
        if self.is_null() {
            return SubclassOf::null();
        }

        world::find_object_raw(class_class(), &self.path)
            .ok()
            .and_then(|h| SubclassOf::from_class(UClassHandle(h.0)).ok())
            .unwrap_or_else(SubclassOf::null)
    }

    #[track_caller]
    pub fn load_synchronous(&self) -> RustealResult<SubclassOf<T>> {
        if self.is_null() {
            return Ok(SubclassOf::null());
        }

        let handle = world::load_object_raw(class_class(), &self.path)?;

        SubclassOf::from_class(UClassHandle(handle.0))
    }
}

impl<T: UeClass> Default for SoftClassRef<T> {
    fn default() -> Self {
        Self::null()
    }
}

impl<T: UeClass> Clone for SoftClassRef<T> {
    fn clone(&self) -> Self {
        Self::new(self.path.clone())
    }
}

impl<T: UeClass> PartialEq for SoftClassRef<T> {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl<T: UeClass> Eq for SoftClassRef<T> {}

impl<T: UeClass> fmt::Debug for SoftClassRef<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SoftClassRef").field(&self.path).finish()
    }
}
