// Marker traits for UE reflection types. Codegen generates impls for all
// exported UClasses, UStructs, and UEnums.

use rusteal_ffi::{UClassHandle, UObjectHandle, UStructHandle};

use crate::error::RustealResult;

/// Implemented by codegen for every exported UClass.
///
/// Provides the static UClass handle needed for cast checks and reflection
/// queries. The handle is typically cached in a `OnceLock` on first access.
pub trait UeClass: 'static {
    /// Get the UClass* for this type (cached after first call).
    fn static_class() -> UClassHandle;
}

/// Implemented by codegen for every exported UScriptStruct.
pub trait UeStruct: 'static {
    /// Get the UScriptStruct* for this type.
    fn static_struct() -> UStructHandle;
}

/// Implemented by codegen for every exported UEnum.
pub trait UeEnum: Copy + 'static {
    /// The underlying integer representation (u8, i32, i64, etc.).
    type Repr: Copy;

    /// The UEnum, typed like a class handle (an enum `#[uproperty]` needs it).
    fn static_enum() -> UClassHandle;

    /// The value as UE stores it in a property.
    fn to_i64(self) -> i64;

    /// The variant for a value UE stored; `None` for a value the enum lacks.
    fn from_i64(value: i64) -> Option<Self>;
}

/// Declares the immediate UE parent class for codegen-exported classes.
///
/// Enables blanket `Deref` impls on `UObjectRef<T>`, `Checked<T>`, and
/// `Pinned<T>` so inherited methods resolve automatically through the
/// Deref chain instead of being flattened into each child's Ext trait.
///
/// Codegen generates `impl HasParent for Pawn { type Parent = Actor; }` etc.
/// Root classes (e.g., `UObject`) do NOT implement this trait.
pub trait HasParent: UeClass {
    type Parent: UeClass;
}

/// `Self` is `U` or one of its subclasses: a `UObjectRef<Self>` is a valid
/// `UObjectRef<U>`, as a `Character*` is an `AActor*` in C++.
///
/// Codegen implements it for every class and each of its ancestors
/// (`impl Inherits<Actor> for Character`); a Rust `#[uclass]` gets it for
/// everything its parent inherits. See `UObjectRef::upcast_to`.
pub trait Inherits<U: UeClass>: UeClass {}

/// Trait for types that hold a UObject handle and can validate it.
///
/// Both `UObjectRef<T>` and `Pinned<T>` implement this, enabling fallible
/// validity checks. For infallible access, use [`ValidHandle`] instead.
pub trait UeHandle {
    /// Return the raw handle if the object is alive, or `Err(ObjectDestroyed)`.
    fn checked_handle(&self) -> RustealResult<UObjectHandle>;

    /// Return the raw handle without validity check.
    fn raw_handle(&self) -> UObjectHandle;
}

/// Trait for types that have been pre-validated and can provide a handle
/// without fallibility. `Checked<T>` and `Pinned<T>` implement this.
///
/// Codegen extension traits use this as a supertrait so that methods
/// return `T` directly instead of `RustealResult<T>`.
pub trait ValidHandle {
    /// Return the raw handle. The implementor guarantees (or debug-asserts)
    /// that the handle is valid.
    fn handle(&self) -> UObjectHandle;
}
