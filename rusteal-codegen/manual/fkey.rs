use rusteal_core::{FName, FNameHandle, OwnedStruct, UStructRef};

use crate::input_core::FKey;

pub trait FKeyExt {
    fn key_name(&self) -> FName;
    fn is_key(&self, name: &str) -> bool;
}

impl FKeyExt for UStructRef<FKey> {
    fn key_name(&self) -> FName {
        unsafe {
            let ptr = self.as_ptr().0 as *const u8;

            FName(FNameHandle(*(ptr as *const u64)))
        }
    }

    fn is_key(&self, name: &str) -> bool {
        self.key_name() == FName::new(name)
    }
}

impl FKey {
    pub fn named(key_name: &str) -> OwnedStruct<FKey> {
        let s = OwnedStruct::<FKey>::new();

        unsafe {
            let ptr = s.as_ref().as_ptr().0 as *mut u8;
            *(ptr as *mut u64) = FName::new(key_name).handle().0;
        }

        s
    }
}
