use std::marker::PhantomData;

use rusteal_ffi::UObjectHandle;

use crate::traits::UeStruct;

pub struct UStructRef<T: UeStruct> {
    ptr: *mut u8,
    _marker: PhantomData<T>,
}

impl<T: UeStruct> UStructRef<T> {
    #[inline]
    pub unsafe fn from_raw(ptr: *mut u8) -> Self {
        UStructRef {
            ptr,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn as_ptr(&self) -> UObjectHandle {
        UObjectHandle(self.ptr as *mut std::ffi::c_void)
    }

    pub fn to_owned(&self) -> crate::containers::OwnedStruct<T> {
        crate::containers::OwnedStruct::copy_from(self)
    }
}

#[inline(always)]
pub unsafe fn struct_ref_from_param<T: UeStruct>(ptr: *mut u8, offset: usize) -> UStructRef<T> {
    unsafe { UStructRef::from_raw(ptr.add(offset)) }
}

impl<T: UeStruct> std::fmt::Debug for UStructRef<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UStructRef")
            .field("ptr", &self.ptr)
            .finish()
    }
}

pub struct OutRef<T: Copy> {
    ptr: *mut T,
}

impl<T: Copy> OutRef<T> {
    #[inline]
    pub fn get(&self) -> T {
        unsafe { self.ptr.read_unaligned() }
    }

    #[inline]
    pub fn set(&self, value: T) {
        unsafe { self.ptr.write_unaligned(value) }
    }
}

#[inline(always)]
pub unsafe fn out_ref_from_param<T: Copy>(ptr: *mut u8, offset: usize) -> OutRef<T> {
    OutRef {
        ptr: unsafe { ptr.add(offset) } as *mut T,
    }
}

impl<T: Copy> std::fmt::Debug for OutRef<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OutRef").field("ptr", &self.ptr).finish()
    }
}
