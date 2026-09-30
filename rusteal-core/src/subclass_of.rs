// SubclassOf<T>: a class reference restricted to `T` and its subclasses, as
// UE's `TSubclassOf<T>`. The property that holds it only accepts such classes,
// so a value read from it is `T` or a subclass (or null).

use std::marker::PhantomData;

use rusteal_ffi::{UClassHandle, UObjectHandle};

use crate::containers::ContainerElement;
use crate::traits::UeClass;

/// A reference to `T`'s class or one of its subclasses; null when unset.
#[repr(transparent)]
pub struct SubclassOf<T: UeClass> {
    handle: UClassHandle,
    _marker: PhantomData<*const T>,
}

// By hand: derives would require the same traits of `T`, which classes lack.
impl<T: UeClass> Clone for SubclassOf<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: UeClass> Copy for SubclassOf<T> {}

impl<T: UeClass> PartialEq for SubclassOf<T> {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}

impl<T: UeClass> Eq for SubclassOf<T> {}

impl<T: UeClass> std::hash::Hash for SubclassOf<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.handle.hash(state);
    }
}

// Send: a class handle is a raw identifier; !Sync as UObjectRef.
unsafe impl<T: UeClass> Send for SubclassOf<T> {}

impl<T: UeClass> SubclassOf<T> {
    /// No class.
    pub fn null() -> Self {
        Self {
            handle: UClassHandle::null(),
            _marker: PhantomData,
        }
    }

    /// `T`'s own class.
    pub fn base() -> Self {
        Self {
            handle: T::static_class(),
            _marker: PhantomData,
        }
    }

    /// Wrap a raw class handle.
    ///
    /// # Safety
    /// The handle must be null or a UClass that is `T` or a subclass of `T`.
    pub unsafe fn from_raw(handle: UClassHandle) -> Self {
        Self {
            handle,
            _marker: PhantomData,
        }
    }

    /// The underlying class handle.
    pub fn raw(&self) -> UClassHandle {
        self.handle
    }

    pub fn is_null(&self) -> bool {
        self.handle.is_null()
    }
}

impl<T: UeClass> Default for SubclassOf<T> {
    fn default() -> Self {
        Self::null()
    }
}

impl<T: UeClass> std::fmt::Debug for SubclassOf<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SubclassOf").field(&self.handle.0).finish()
    }
}

// A class is a UObject: in a container it travels as an 8-byte object pointer.
unsafe impl<T: UeClass> ContainerElement for SubclassOf<T> {
    const BUF_SIZE: u32 = std::mem::size_of::<UObjectHandle>() as u32;

    #[inline]
    unsafe fn read_from_buf(buf: *const u8, _written: u32) -> Self {
        unsafe {
            let handle = (buf as *const UObjectHandle).read_unaligned();
            Self::from_raw(UClassHandle(handle.0))
        }
    }

    #[inline]
    unsafe fn write_to_buf(&self, buf: *mut u8) -> u32 {
        unsafe {
            (buf as *mut UObjectHandle).write_unaligned(UObjectHandle(self.handle.0));
            std::mem::size_of::<UObjectHandle>() as u32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Probe;
    impl UeClass for Probe {
        fn static_class() -> UClassHandle {
            UClassHandle::from_addr(0x1000)
        }
    }

    #[test]
    fn compares_by_class() {
        let base = SubclassOf::<Probe>::base();
        assert_eq!(base, SubclassOf::<Probe>::base());
        assert_ne!(base, SubclassOf::<Probe>::null());
        assert!(SubclassOf::<Probe>::default().is_null());
        assert_eq!(base.raw(), Probe::static_class());
    }

    #[test]
    fn round_trips_through_a_container_buffer() {
        let base = SubclassOf::<Probe>::base();
        let mut buf = [0u8; 8];
        unsafe {
            assert_eq!(base.write_to_buf(buf.as_mut_ptr()), 8);
            assert_eq!(SubclassOf::<Probe>::read_from_buf(buf.as_ptr(), 8), base);
        }
    }
}
