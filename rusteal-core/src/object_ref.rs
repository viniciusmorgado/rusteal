use std::marker::PhantomData;
use std::ops::Deref;
use std::panic::Location;

use rusteal_ffi::{FWeakObjectHandle, UClassHandle, UObjectHandle};

use crate::error::{RustealError, RustealResult};
use crate::ffi_dispatch;
use crate::pinned::Pinned;
use crate::script_interface::ScriptInterface;
use crate::traits::{HasParent, UeClass, UeHandle, UeInterface, ValidHandle};

/// The index and serial number of the object at `handle`, which must be null or an object the
/// engine has just given.
#[doc(hidden)]
#[inline]
pub unsafe fn weak_handle_of(handle: UObjectHandle) -> FWeakObjectHandle {
    if handle.is_null() {
        FWeakObjectHandle::default()
    } else {
        unsafe { ffi_dispatch::core_make_weak(handle) }
    }
}

/// The address of the object `weak` names while it lives, null once it is destroyed. Never reads
/// the object's memory: the engine's object array is looked up by index and the serial compared.
#[doc(hidden)]
#[inline]
pub fn resolve_weak_handle(weak: FWeakObjectHandle) -> UObjectHandle {
    if weak.object_serial_number == 0 {
        return UObjectHandle::null();
    }

    unsafe { ffi_dispatch::core_resolve_weak(weak) }
}

/// A reference to an engine object, kept as the object's index and serial number in the engine's
/// object array, the way `FWeakObjectPtr` keeps it. It never holds the object's address, so it
/// cannot read freed memory or mistake a new object at a reused address for the old one: once
/// the object is destroyed, [`is_valid`](Self::is_valid) is false and
/// [`checked`](Self::checked) is an error.
#[repr(transparent)]
pub struct UObjectRef<T: UeClass> {
    weak: FWeakObjectHandle,
    _marker: PhantomData<*const T>,
}

impl<T: UeClass> Clone for UObjectRef<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: UeClass> Copy for UObjectRef<T> {}

impl<T: UeClass> PartialEq for UObjectRef<T> {
    fn eq(&self, other: &Self) -> bool {
        self.weak == other.weak
    }
}

impl<T: UeClass> Eq for UObjectRef<T> {}

impl<T: UeClass> std::hash::Hash for UObjectRef<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.weak.hash(state);
    }
}

unsafe impl<T: UeClass> Send for UObjectRef<T> {}

impl<T: UeClass> Default for UObjectRef<T> {
    fn default() -> Self {
        Self::null()
    }
}

impl<T: UeClass> UObjectRef<T> {
    #[inline]
    pub fn null() -> Self {
        UObjectRef {
            weak: FWeakObjectHandle::default(),
            _marker: PhantomData,
        }
    }

    /// The reference to the object at `handle`, which must be null or an object the engine has
    /// just given: its index and serial number are read from it now.
    #[inline]
    pub unsafe fn from_raw(handle: UObjectHandle) -> Self {
        UObjectRef {
            weak: unsafe { weak_handle_of(handle) },
            _marker: PhantomData,
        }
    }

    /// The reference an index and serial number name; it is valid while that object lives.
    #[inline]
    pub fn from_weak(weak: FWeakObjectHandle) -> Self {
        UObjectRef {
            weak,
            _marker: PhantomData,
        }
    }

    /// The object's index and serial number in the engine's object array.
    #[inline]
    pub fn weak_handle(&self) -> FWeakObjectHandle {
        self.weak
    }

    /// The object's address while it lives, null once it is destroyed.
    #[inline]
    pub fn raw(&self) -> UObjectHandle {
        resolve_weak_handle(self.weak)
    }

    #[inline]
    pub fn is_valid(&self) -> bool {
        !self.raw().is_null()
    }

    #[inline]
    #[track_caller]
    pub fn checked(&self) -> RustealResult<Checked<T>> {
        let handle = self.raw();

        if handle.is_null() {
            Err(crate::error::destroyed::<T>(Location::caller()))
        } else {
            Ok(Checked {
                handle,
                _marker: PhantomData,
            })
        }
    }

    #[track_caller]
    pub fn cast<U: UeClass>(self) -> RustealResult<UObjectRef<U>> {
        let h = self.checked()?.raw();
        let target = U::static_class();
        if unsafe { ffi_dispatch::core_is_a(h, target) } {
            Ok(UObjectRef {
                weak: self.weak,
                _marker: PhantomData,
            })
        } else {
            Err(crate::error::cast_failed(h, target, Location::caller()))
        }
    }

    #[track_caller]
    pub fn pin(self) -> RustealResult<Pinned<T>> {
        Pinned::new(self)
    }

    #[track_caller]
    pub fn get_name(&self) -> RustealResult<String> {
        let h = self.checked()?.raw();

        crate::error::object_name(h).ok_or_else(|| RustealError::object_destroyed().on_handle(h))
    }

    #[track_caller]
    pub fn get_class(&self) -> RustealResult<UClassHandle> {
        let h = self.checked()?.raw();

        Ok(unsafe { ffi_dispatch::core_get_class(h) })
    }

    #[track_caller]
    pub fn get_outer(&self) -> RustealResult<UObjectHandle> {
        let h = self.checked()?.raw();

        Ok(unsafe { ffi_dispatch::core_get_outer(h) })
    }

    #[track_caller]
    pub fn from_handle(handle: UObjectHandle) -> RustealResult<Self> {
        if !unsafe { ffi_dispatch::core_is_valid(handle) } {
            return Err(RustealError::object_destroyed());
        }

        if !unsafe { ffi_dispatch::core_is_a(handle, T::static_class()) } {
            return Err(RustealError::invalid_cast_to(handle, T::static_class()));
        }

        Ok(unsafe { Self::from_raw(handle) })
    }

    #[track_caller]
    pub fn class(&self) -> RustealResult<crate::subclass_of::SubclassOf<T>> {
        let class = self.get_class()?;

        Ok(unsafe { crate::subclass_of::SubclassOf::from_raw(class) })
    }

    #[track_caller]
    pub fn outer<U: UeClass>(&self) -> RustealResult<UObjectRef<U>> {
        UObjectRef::from_handle(self.get_outer()?)
    }

    #[inline]
    pub fn is_a<U: UeClass>(&self) -> bool {
        let handle = self.raw();

        !handle.is_null() && unsafe { ffi_dispatch::core_is_a(handle, U::static_class()) }
    }

    #[inline]
    pub fn implements<I: UeInterface + ?Sized>(&self) -> bool {
        let handle = self.raw();

        !handle.is_null() && unsafe { ffi_dispatch::core_is_a(handle, I::static_class()) }
    }

    pub fn interface<I: UeInterface + ?Sized>(self) -> RustealResult<ScriptInterface<I>> {
        ScriptInterface::new(self.checked()?.raw())
    }
}

impl<T: HasParent> UObjectRef<T> {
    #[inline]
    pub fn upcast(self) -> UObjectRef<T::Parent> {
        UObjectRef::from_weak(self.weak)
    }
}

impl<T: UeClass> UObjectRef<T> {
    #[inline]
    pub fn upcast_to<U: UeClass>(self) -> UObjectRef<U>
    where
        T: crate::traits::Inherits<U>,
    {
        UObjectRef::from_weak(self.weak)
    }
}

pub trait ObjectPointer: Sized {
    unsafe fn from_object_handle(handle: UObjectHandle) -> Self;

    fn object_handle(&self) -> UObjectHandle;
}

impl<T: UeClass> ObjectPointer for UObjectRef<T> {
    #[inline]
    unsafe fn from_object_handle(handle: UObjectHandle) -> Self {
        unsafe { UObjectRef::from_raw(handle) }
    }

    #[inline]
    fn object_handle(&self) -> UObjectHandle {
        self.raw()
    }
}

impl<T: HasParent> Deref for UObjectRef<T> {
    type Target = UObjectRef<T::Parent>;
    #[inline]
    fn deref(&self) -> &UObjectRef<T::Parent> {
        unsafe { &*(self as *const _ as *const UObjectRef<T::Parent>) }
    }
}

impl<T: UeClass> UeHandle for UObjectRef<T> {
    #[inline]
    fn checked_handle(&self) -> RustealResult<UObjectHandle> {
        self.checked().map(|c| c.raw())
    }

    #[inline]
    fn raw_handle(&self) -> UObjectHandle {
        self.raw()
    }
}

impl<T: UeClass> std::fmt::Debug for UObjectRef<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UObjectRef")
            .field("index", &self.weak.object_index)
            .field("serial", &self.weak.object_serial_number)
            .field("valid", &self.is_valid())
            .finish()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Checked<T: UeClass> {
    handle: UObjectHandle,
    _marker: PhantomData<*const T>,
}

unsafe impl<T: UeClass> Send for Checked<T> {}

impl<T: UeClass> Checked<T> {
    #[inline]
    pub(crate) fn new_unchecked(handle: UObjectHandle) -> Self {
        Checked {
            handle,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn raw(&self) -> UObjectHandle {
        self.handle
    }

    #[inline]
    pub fn as_ref(&self) -> UObjectRef<T> {
        unsafe { UObjectRef::from_raw(self.handle) }
    }

    #[inline]
    pub fn is_a<U: UeClass>(&self) -> bool {
        unsafe { ffi_dispatch::core_is_a(self.handle, U::static_class()) }
    }

    #[track_caller]
    pub fn cast<U: UeClass>(self) -> RustealResult<Checked<U>> {
        if self.is_a::<U>() {
            Ok(Checked::new_unchecked(self.handle))
        } else {
            Err(crate::error::cast_failed(
                self.handle,
                U::static_class(),
                Location::caller(),
            ))
        }
    }
}

impl<T: HasParent> Checked<T> {
    #[inline]
    pub fn upcast(self) -> Checked<T::Parent> {
        Checked::new_unchecked(self.handle)
    }
}

impl<T: HasParent> Deref for Checked<T> {
    type Target = Checked<T::Parent>;
    #[inline]
    fn deref(&self) -> &Checked<T::Parent> {
        unsafe { &*(self as *const _ as *const Checked<T::Parent>) }
    }
}

impl<T: UeClass> ValidHandle for Checked<T> {
    #[inline]
    fn handle(&self) -> UObjectHandle {
        self.handle
    }
}

impl<T: UeClass> std::fmt::Debug for Checked<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Checked")
            .field("handle", &self.handle)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Probe;

    impl UeClass for Probe {
        fn static_class() -> UClassHandle {
            UClassHandle::null()
        }
    }

    fn named(object_index: i32, object_serial_number: i32) -> UObjectRef<Probe> {
        UObjectRef::from_weak(FWeakObjectHandle {
            object_index,
            object_serial_number,
        })
    }

    #[test]
    fn a_reference_is_the_size_of_an_index_and_a_serial_number() {
        assert_eq!(std::mem::size_of::<UObjectRef<Probe>>(), 8);
        assert_eq!(
            std::mem::size_of::<UObjectRef<Probe>>(),
            std::mem::size_of::<FWeakObjectHandle>()
        );
    }

    #[test]
    fn a_null_reference_is_invalid_without_asking_the_engine() {
        // No API table is installed here: resolving through it would panic.
        let null = UObjectRef::<Probe>::null();

        assert!(!null.is_valid());
        assert!(null.raw().is_null());
        assert!(null.checked().is_err());
        assert_eq!(null, UObjectRef::default());
    }

    #[test]
    fn identity_is_the_index_and_the_serial_number() {
        assert_eq!(named(7, 3), named(7, 3));
        assert_ne!(named(7, 3), named(7, 4), "a reused slot is another object");
        assert_ne!(named(7, 3), named(8, 3));

        let set: std::collections::HashSet<_> = [named(7, 3), named(7, 3), named(7, 4)].into();
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn a_weak_pointer_and_a_reference_share_the_handle() {
        let reference = named(12, 9);
        let weak = crate::weak_ptr::TWeakObjectPtr::from_ref(&reference);

        assert_eq!(weak.handle(), reference.weak_handle());
    }
}
