use rusteal_core::{OwnedStruct, UStructHandle, UStructRef, UeStruct};

use crate::core_ue::FInstancedStruct;

pub trait InstancedStructExt {
    fn script_struct(&self) -> UStructHandle;

    fn is_set(&self) -> bool;

    fn get<T: UeStruct>(&self) -> Option<OwnedStruct<T>>;

    fn set<T: UeStruct>(&self, value: &OwnedStruct<T>);

    fn reset(&self);
}

impl InstancedStructExt for UStructRef<FInstancedStruct> {
    fn script_struct(&self) -> UStructHandle {
        unsafe { rusteal_core::ffi_dispatch::reflection_instanced_struct_type(self.as_ptr().0 as *const u8) }
    }

    fn is_set(&self) -> bool {
        !self.script_struct().is_null()
    }

    fn get<T: UeStruct>(&self) -> Option<OwnedStruct<T>> {
        let size = unsafe { rusteal_core::ffi_dispatch::reflection_get_struct_size(T::static_struct()) };
        let mut buf = vec![0u8; size as usize];

        let found = unsafe {
            rusteal_core::ffi_dispatch::reflection_instanced_struct_get(
                self.as_ptr().0 as *const u8,
                T::static_struct(),
                buf.as_mut_ptr(),
            )
        };

        found.then(|| unsafe { OwnedStruct::from_constructed(buf) })
    }

    fn set<T: UeStruct>(&self, value: &OwnedStruct<T>) {
        unsafe {
            rusteal_core::ffi_dispatch::reflection_instanced_struct_set(
                self.as_ptr().0 as *mut u8,
                T::static_struct(),
                value.as_bytes().as_ptr(),
            );
        }
    }

    fn reset(&self) {
        unsafe {
            rusteal_core::ffi_dispatch::reflection_instanced_struct_reset(self.as_ptr().0 as *mut u8);
        }
    }
}

impl FInstancedStruct {
    pub fn make<T: UeStruct>(value: &OwnedStruct<T>) -> OwnedStruct<FInstancedStruct> {
        let instanced = OwnedStruct::<FInstancedStruct>::new();
        instanced.as_ref().set(value);

        instanced
    }
}
