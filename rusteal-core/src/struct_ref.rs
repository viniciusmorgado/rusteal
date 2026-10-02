// UStructRef<T>: lightweight typed wrapper for struct memory pointers.
//
// Unlike UObjectRef<T>, struct memory is not garbage-collected by UE.
// UStructRef is just a typed raw pointer to a struct instance in memory
// (e.g., inside a UObject property or a parameter buffer).

use std::marker::PhantomData;

use rusteal_ffi::UObjectHandle;

use crate::traits::UeStruct;

/// A typed, non-owning reference to a UE struct instance in memory.
///
/// This wraps a raw pointer to struct data (e.g., an FVector stored inside
/// a UObject property). The struct memory is managed by its container —
/// no validity check is needed (unlike UObjectRef).
///
/// PropertyApi methods accept `UObjectHandle` (`*mut c_void`) which works
/// for both UObject pointers and raw struct memory pointers.
pub struct UStructRef<T: UeStruct> {
    ptr: *mut u8,
    _marker: PhantomData<T>,
}

impl<T: UeStruct> UStructRef<T> {
    /// Create from a raw pointer to struct memory.
    ///
    /// # Safety
    /// The caller must ensure `ptr` points to valid memory containing a `T`.
    #[inline]
    pub unsafe fn from_raw(ptr: *mut u8) -> Self {
        UStructRef {
            ptr,
            _marker: PhantomData,
        }
    }

    /// Get the raw pointer as a `UObjectHandle`.
    ///
    /// PropertyApi methods take `UObjectHandle` which is `*mut c_void` —
    /// this works for both UObject pointers and raw struct memory.
    #[inline]
    pub fn as_ptr(&self) -> UObjectHandle {
        UObjectHandle(self.ptr as *mut std::ffi::c_void)
    }

    /// An owned copy of the struct, for the generated functions that take
    /// `&OwnedStruct<T>` or to keep it past the call it came with.
    pub fn to_owned(&self) -> crate::containers::OwnedStruct<T> {
        crate::containers::OwnedStruct::copy_from(self)
    }
}

/// Create a `UStructRef<T>` from a native parameter buffer pointer + byte offset.
///
/// Used by `#[uclass_impl]` macro for Override function struct parameters.
///
/// # Safety
///
/// `ptr` must point to a live parameter buffer and `offset` must be the offset
/// of a `T` inside it, as UE laid the buffer out; the reference must not outlive
/// the call the buffer belongs to.
#[inline(always)]
pub unsafe fn struct_ref_from_param<T: UeStruct>(ptr: *mut u8, offset: usize) -> UStructRef<T> {
    unsafe { UStructRef::from_raw(ptr.add(offset)) }
}

impl<T: UeStruct> std::fmt::Debug for UStructRef<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UStructRef").field("ptr", &self.ptr).finish()
    }
}

/// A scalar out parameter of an engine event a `#[ufunction(Override)]`
/// implements (`float& NewCameraFOV`): `set` writes the value the engine reads
/// back once the call returns. Struct out parameters are `UStructRef`s, written
/// through their setters the same way.
pub struct OutRef<T: Copy> {
    ptr: *mut T,
}

impl<T: Copy> OutRef<T> {
    /// The value the caller passed in.
    #[inline]
    pub fn get(&self) -> T {
        unsafe { self.ptr.read_unaligned() }
    }

    /// The value the caller gets back.
    #[inline]
    pub fn set(&self, value: T) {
        unsafe { self.ptr.write_unaligned(value) }
    }
}

/// Create an `OutRef<T>` from a native parameter buffer pointer + byte offset.
///
/// Used by `#[uclass_impl]` for Override out parameters.
///
/// # Safety
///
/// As [`struct_ref_from_param`]: `offset` is a `T` inside the live buffer `ptr`.
#[inline(always)]
pub unsafe fn out_ref_from_param<T: Copy>(ptr: *mut u8, offset: usize) -> OutRef<T> {
    OutRef { ptr: unsafe { ptr.add(offset) } as *mut T }
}

impl<T: Copy> std::fmt::Debug for OutRef<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OutRef").field("ptr", &self.ptr).finish()
    }
}
