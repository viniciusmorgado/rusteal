// Prelude: one-import access to the most commonly used runtime types.
//
// Usage: `use rusteal_runtime::prelude::*;`. The engine types live in the
// project's generated `bindings` crate, which has its own `bindings::prelude`.

// Core runtime types
pub use rusteal_core::{
    UObjectRef, Pinned, RustealResult, RustealError, UeClass, UeStruct, UeEnum,
    OwnedStruct, UStructRef, UeArray, UeMap, UeSet,
    DynamicCall, DynamicCallResult, DelegateBinding,
    FName, TWeakObjectPtr,
    LOG_DISPLAY, LOG_WARNING, LOG_ERROR,
};

// UE math types (rusteal-core)
pub use rusteal_core::{
    Rotator, Transform, LinearColor, Color,
    Plane, Ray, Sphere, UeBox, UeBox2d, BoxSphereBounds,
};

// FFI handles (rarely needed directly, but useful for advanced cases)
pub use rusteal_core::{UObjectHandle, UClassHandle, FPropertyHandle, UStructHandle, FNameHandle};

// Proc macros
pub use rusteal_macros::{uclass, uclass_impl};

// glam re-exports (common math types users will interact with)
pub use glam::{DVec2, DVec3, DVec4, DQuat, DMat4, IVec2, IVec3};
