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
///
/// `'a` is the call that handed the reference out: an engine event, a function, a callback. The
/// reference cannot outlive it, so it cannot be stored where the garbage collector does not see
/// it (a struct field, a static, a closure or a future kept for later). To keep an object, use a
/// `#[uproperty]`, a [`TWeakObjectPtr`](crate::TWeakObjectPtr) or a [`Pinned`].
#[repr(transparent)]
pub struct UObjectRef<'a, T: UeClass> {
    weak: FWeakObjectHandle,
    _marker: PhantomData<(&'a (), *const T)>,
}

impl<T: UeClass> Clone for UObjectRef<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: UeClass> Copy for UObjectRef<'_, T> {}

impl<T: UeClass> PartialEq for UObjectRef<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        self.weak == other.weak
    }
}

impl<T: UeClass> Eq for UObjectRef<'_, T> {}

impl<T: UeClass> std::hash::Hash for UObjectRef<'_, T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.weak.hash(state);
    }
}

impl<T: UeClass> Default for UObjectRef<'_, T> {
    fn default() -> Self {
        Self::null()
    }
}

impl<'a, T: UeClass> UObjectRef<'a, T> {
    #[inline]
    pub fn null() -> Self {
        UObjectRef {
            weak: FWeakObjectHandle::default(),
            _marker: PhantomData,
        }
    }

    /// The reference to the object at `handle`, which must be null or an object the engine has
    /// just given: its index and serial number are read from it now. The caller picks `'a`, so it
    /// must not outlive the call that gave `handle`.
    #[inline]
    pub unsafe fn from_raw(handle: UObjectHandle) -> Self {
        UObjectRef {
            weak: unsafe { weak_handle_of(handle) },
            _marker: PhantomData,
        }
    }

    /// The reference an index and serial number name, for as long as the caller lets it live.
    #[inline]
    pub(crate) fn from_weak(weak: FWeakObjectHandle) -> Self {
        UObjectRef {
            weak,
            _marker: PhantomData,
        }
    }

    /// The same reference, for as long as `anchor` may live: both were handed out by calls that
    /// are still running, so one can stand in for the other. It is how a function returns an
    /// object it read through a temporary, such as a property of a subsystem found for the call:
    /// `tour.hint_widget().with_scope_of(&me)`.
    #[inline]
    pub fn with_scope_of<'b>(self, anchor: &UObjectRef<'b, impl UeClass>) -> UObjectRef<'b, T> {
        let _ = anchor;

        UObjectRef::from_weak(self.weak)
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

    /// The live object, to call the engine on; it does not borrow the reference, so it lives as
    /// long as the reference may.
    #[inline]
    #[track_caller]
    pub fn checked(&self) -> RustealResult<Checked<'a, T>> {
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
    pub fn cast<U: UeClass>(self) -> RustealResult<UObjectRef<'a, U>> {
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

    /// The reference to the object at `handle`, checked to be alive and a `T`. Like
    /// [`from_raw`](Self::from_raw), the caller picks `'a`.
    #[track_caller]
    pub unsafe fn from_handle(handle: UObjectHandle) -> RustealResult<Self> {
        if handle.is_null() || !unsafe { ffi_dispatch::core_is_valid(handle) } {
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
    pub fn outer<U: UeClass>(&self) -> RustealResult<UObjectRef<'a, U>> {
        unsafe { UObjectRef::from_handle(self.get_outer()?) }
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

    pub fn interface<I: UeInterface + ?Sized>(self) -> RustealResult<ScriptInterface<'a, I>> {
        ScriptInterface::new(self.checked()?.raw())
    }
}

impl<'a, T: HasParent> UObjectRef<'a, T> {
    #[inline]
    pub fn upcast(self) -> UObjectRef<'a, T::Parent> {
        UObjectRef::from_weak(self.weak)
    }
}

impl<'a, T: UeClass> UObjectRef<'a, T> {
    #[inline]
    pub fn upcast_to<U: UeClass>(self) -> UObjectRef<'a, U>
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

impl<T: UeClass> ObjectPointer for UObjectRef<'_, T> {
    #[inline]
    unsafe fn from_object_handle(handle: UObjectHandle) -> Self {
        unsafe { UObjectRef::from_raw(handle) }
    }

    #[inline]
    fn object_handle(&self) -> UObjectHandle {
        self.raw()
    }
}

impl<'a, T: HasParent> Deref for UObjectRef<'a, T> {
    type Target = UObjectRef<'a, T::Parent>;
    #[inline]
    fn deref(&self) -> &UObjectRef<'a, T::Parent> {
        unsafe { &*(self as *const _ as *const UObjectRef<'a, T::Parent>) }
    }
}

impl<T: UeClass> UeHandle for UObjectRef<'_, T> {
    #[inline]
    fn checked_handle(&self) -> RustealResult<UObjectHandle> {
        self.checked().map(|c| c.raw())
    }

    #[inline]
    fn raw_handle(&self) -> UObjectHandle {
        self.raw()
    }
}

impl<T: UeClass> std::fmt::Debug for UObjectRef<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UObjectRef")
            .field("index", &self.weak.object_index)
            .field("serial", &self.weak.object_serial_number)
            .field("valid", &self.is_valid())
            .finish()
    }
}

/// A live object, checked by [`UObjectRef::checked`], with the engine's functions on it through
/// the generated `...Ext` traits. It lives no longer than the reference it came from.
#[repr(transparent)]
pub struct Checked<'a, T: UeClass> {
    handle: UObjectHandle,
    _marker: PhantomData<(&'a (), *const T)>,
}

impl<T: UeClass> Clone for Checked<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: UeClass> Copy for Checked<'_, T> {}

impl<T: UeClass> PartialEq for Checked<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}

impl<T: UeClass> Eq for Checked<'_, T> {}

impl<T: UeClass> std::hash::Hash for Checked<'_, T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.handle.hash(state);
    }
}

impl<'a, T: UeClass> Checked<'a, T> {
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
    pub fn as_ref(&self) -> UObjectRef<'a, T> {
        unsafe { UObjectRef::from_raw(self.handle) }
    }

    #[inline]
    pub fn is_a<U: UeClass>(&self) -> bool {
        unsafe { ffi_dispatch::core_is_a(self.handle, U::static_class()) }
    }

    #[track_caller]
    pub fn cast<U: UeClass>(self) -> RustealResult<Checked<'a, U>> {
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

impl<'a, T: HasParent> Checked<'a, T> {
    #[inline]
    pub fn upcast(self) -> Checked<'a, T::Parent> {
        Checked::new_unchecked(self.handle)
    }
}

impl<'a, T: HasParent> Deref for Checked<'a, T> {
    type Target = Checked<'a, T::Parent>;
    #[inline]
    fn deref(&self) -> &Checked<'a, T::Parent> {
        unsafe { &*(self as *const _ as *const Checked<'a, T::Parent>) }
    }
}

impl<T: UeClass> ValidHandle for Checked<'_, T> {
    #[inline]
    fn handle(&self) -> UObjectHandle {
        self.handle
    }
}

impl<T: UeClass> std::fmt::Debug for Checked<'_, T> {
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

    fn named(object_index: i32, object_serial_number: i32) -> UObjectRef<'static, Probe> {
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
