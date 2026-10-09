use rusteal_core::UeEnum;

use crate::engine::{ECollisionChannel, EObjectTypeQuery};

pub fn object_type_query(channel: ECollisionChannel) -> Option<EObjectTypeQuery> {
    let value =
        unsafe { rusteal_core::ffi_dispatch::world_channel_to_object_type(channel.to_i64() as u8) };

    EObjectTypeQuery::from_value(value)
}
