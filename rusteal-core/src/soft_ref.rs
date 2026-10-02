// SoftObjectRef<T>: UE's TSoftObjectPtr<T>, a reference to an asset by path
// that does not keep it loaded.

use std::fmt;
use std::marker::PhantomData;

use crate::error::RustealResult;
use crate::object_ref::UObjectRef;
use crate::traits::UeClass;
use crate::world;

/// A soft reference to an object of class `T`: its path
/// (`/Game/Meshes/SM_Rifle.SM_Rifle`), loaded on demand. A `#[uproperty]` or
/// `#[ustruct]` field of this type is a `TSoftObjectPtr<T>`.
pub struct SoftObjectRef<T: UeClass> {
    path: String,
    _marker: PhantomData<T>,
}

impl<T: UeClass> SoftObjectRef<T> {
    /// A reference to the object at `path`; empty for none.
    pub fn new(path: impl Into<String>) -> Self {
        SoftObjectRef {
            path: path.into(),
            _marker: PhantomData,
        }
    }

    /// No reference.
    pub fn null() -> Self {
        Self::new(String::new())
    }

    /// The object's path, empty for none.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Whether it refers to nothing.
    pub fn is_null(&self) -> bool {
        self.path.is_empty()
    }

    /// The object if it is loaded, without loading it (`TSoftObjectPtr::Get`).
    pub fn get(&self) -> UObjectRef<T> {
        if self.is_null() {
            return UObjectRef::null();
        }
        world::find_object_raw(T::static_class(), &self.path)
            .map(|h| unsafe { UObjectRef::from_raw(h) })
            .unwrap_or_else(|_| UObjectRef::null())
    }

    /// The object, loaded if it is not (`TSoftObjectPtr::LoadSynchronous`); a
    /// null reference for none.
    pub fn load_synchronous(&self) -> RustealResult<UObjectRef<T>> {
        if self.is_null() {
            return Ok(UObjectRef::null());
        }
        let handle = world::load_object_raw(T::static_class(), &self.path)?;
        Ok(unsafe { UObjectRef::from_raw(handle) })
    }
}

impl<T: UeClass> Default for SoftObjectRef<T> {
    fn default() -> Self {
        Self::null()
    }
}

impl<T: UeClass> Clone for SoftObjectRef<T> {
    fn clone(&self) -> Self {
        Self::new(self.path.clone())
    }
}

impl<T: UeClass> PartialEq for SoftObjectRef<T> {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl<T: UeClass> Eq for SoftObjectRef<T> {}

impl<T: UeClass> fmt::Debug for SoftObjectRef<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SoftObjectRef").field(&self.path).finish()
    }
}
