use std::ffi::c_void;

macro_rules! define_ptr_handle {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        pub struct $name(pub *mut c_void);

        impl $name {
            pub fn is_null(&self) -> bool {
                self.0.is_null()
            }

            pub fn null() -> Self {
                Self(std::ptr::null_mut())
            }

            pub fn from_addr(addr: u64) -> Self {
                Self(addr as usize as *mut c_void)
            }

            pub fn to_addr(&self) -> u64 {
                self.0 as usize as u64
            }
        }
    };
}

define_ptr_handle! {
    UObjectHandle
}

define_ptr_handle! {
    UClassHandle
}

define_ptr_handle! {
    FPropertyHandle
}

define_ptr_handle! {
    UFunctionHandle
}

define_ptr_handle! {
    UStructHandle
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct FNameHandle(pub u64);

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FWeakObjectHandle {
    pub object_index: i32,
    pub object_serial_number: i32,
}

impl Default for FWeakObjectHandle {
    fn default() -> Self {
        FWeakObjectHandle {
            object_index: -1,
            object_serial_number: 0,
        }
    }
}

unsafe impl Send for UObjectHandle {}
unsafe impl Sync for UObjectHandle {}
unsafe impl Send for UClassHandle {}
unsafe impl Sync for UClassHandle {}
unsafe impl Send for FPropertyHandle {}
unsafe impl Sync for FPropertyHandle {}
unsafe impl Send for UFunctionHandle {}
unsafe impl Sync for UFunctionHandle {}
unsafe impl Send for UStructHandle {}
unsafe impl Sync for UStructHandle {}
unsafe impl Send for FWeakObjectHandle {}
unsafe impl Sync for FWeakObjectHandle {}
