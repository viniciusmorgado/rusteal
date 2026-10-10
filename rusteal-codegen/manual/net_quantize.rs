use glam::DVec3;
use rusteal_core::{OwnedStruct, UeStruct};

use crate::engine::{
    FVector_NetQuantize, FVector_NetQuantize10, FVector_NetQuantize100,
    FVector_NetQuantizeNormal,
};
use crate::manual::vector::OwnedFVectorExt;

fn read_xyz<T: UeStruct>(value: &OwnedStruct<T>) -> DVec3 {
    let read = |name: &str| {
        let prop = unsafe {
            rusteal_core::ffi_dispatch::reflection_find_struct_property(
                T::static_struct(),
                name.as_ptr(),
                name.len() as u32,
            )
        };

        let mut out = 0.0f64;

        rusteal_core::ffi_infallible_ctx(
            unsafe { rusteal_core::ffi_dispatch::property_get_f64(value.as_ref().as_ptr(), prop, &mut out) },
            name,
        );

        out
    };

    DVec3::new(read("X"), read("Y"), read("Z"))
}

macro_rules! vector_like {
    ($($ty:ty),*) => {$(
        impl OwnedFVectorExt for OwnedStruct<$ty> {
            fn to_dvec3(&self) -> DVec3 {
                read_xyz(self)
            }
        }
    )*};
}

vector_like!(
    FVector_NetQuantize,
    FVector_NetQuantize10,
    FVector_NetQuantize100,
    FVector_NetQuantizeNormal
);
