// RustealPropertyApiImpl.cpp — FRustealPropertyApi implementation.

#include "RustealApiTable.h"
#include "UObject/UnrealType.h"
#include "UObject/TextProperty.h"
#include "UObject/UObjectGlobals.h"

// ---------------------------------------------------------------------------
// Validity macro
// ---------------------------------------------------------------------------

// NOTE: The property API is used for both UObject properties and raw struct
// field access (OwnedStruct buffers). We only null-check the container
// pointer; UObject validity is the caller's responsibility. Using void*
// ensures ContainerPtrToValuePtr uses the offset-only overload (no
// IsValidLowLevel assertion).
#define RUSTEAL_CHECK_VALID(ObjHandle)                                     \
    void* Object = (ObjHandle).ptr;                                     \
    if (!Object)                                                        \
    {                                                                   \
        return ERustealErrorCode::ObjectDestroyed;                         \
    }

// ---------------------------------------------------------------------------
// Bool (bit-field safe via FBoolProperty)
// ---------------------------------------------------------------------------

static ERustealErrorCode GetBoolImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop, bool* Out)
{
    RUSTEAL_CHECK_VALID(Obj);
    const FBoolProperty* BoolProp = CastField<FBoolProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!BoolProp)
    {
        return ERustealErrorCode::TypeMismatch;
    }
    *Out = BoolProp->GetPropertyValue_InContainer(Object);
    return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetBoolImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop, bool Val)
{
    RUSTEAL_CHECK_VALID(Obj);
    FBoolProperty* BoolProp = CastField<FBoolProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!BoolProp)
    {
        return ERustealErrorCode::TypeMismatch;
    }
    BoolProp->SetPropertyValue_InContainer(Object, Val);
    return ERustealErrorCode::Ok;
}

// ---------------------------------------------------------------------------
// Int32
// ---------------------------------------------------------------------------

static ERustealErrorCode GetI32Impl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop, int32* Out)
{
    RUSTEAL_CHECK_VALID(Obj);
    FProperty* Property = static_cast<FProperty*>(Prop.ptr);
    *Out = *Property->ContainerPtrToValuePtr<int32>(Object);
    return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetI32Impl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop, int32 Val)
{
    RUSTEAL_CHECK_VALID(Obj);
    FProperty* Property = static_cast<FProperty*>(Prop.ptr);
    *Property->ContainerPtrToValuePtr<int32>(Object) = Val;
    return ERustealErrorCode::Ok;
}

// ---------------------------------------------------------------------------
// Int64
// ---------------------------------------------------------------------------

static ERustealErrorCode GetI64Impl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop, int64* Out)
{
    RUSTEAL_CHECK_VALID(Obj);
    FProperty* Property = static_cast<FProperty*>(Prop.ptr);
    *Out = *Property->ContainerPtrToValuePtr<int64>(Object);
    return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetI64Impl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop, int64 Val)
{
    RUSTEAL_CHECK_VALID(Obj);
    FProperty* Property = static_cast<FProperty*>(Prop.ptr);
    *Property->ContainerPtrToValuePtr<int64>(Object) = Val;
    return ERustealErrorCode::Ok;
}

// ---------------------------------------------------------------------------
// UInt8
// ---------------------------------------------------------------------------

static ERustealErrorCode GetU8Impl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop, uint8* Out)
{
    RUSTEAL_CHECK_VALID(Obj);
    FProperty* Property = static_cast<FProperty*>(Prop.ptr);
    *Out = *Property->ContainerPtrToValuePtr<uint8>(Object);
    return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetU8Impl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop, uint8 Val)
{
    RUSTEAL_CHECK_VALID(Obj);
    FProperty* Property = static_cast<FProperty*>(Prop.ptr);
    *Property->ContainerPtrToValuePtr<uint8>(Object) = Val;
    return ERustealErrorCode::Ok;
}

// ---------------------------------------------------------------------------
// Float
// ---------------------------------------------------------------------------

static ERustealErrorCode GetF32Impl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop, float* Out)
{
    RUSTEAL_CHECK_VALID(Obj);
    FProperty* Property = static_cast<FProperty*>(Prop.ptr);
    *Out = *Property->ContainerPtrToValuePtr<float>(Object);
    return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetF32Impl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop, float Val)
{
    RUSTEAL_CHECK_VALID(Obj);
    FProperty* Property = static_cast<FProperty*>(Prop.ptr);
    *Property->ContainerPtrToValuePtr<float>(Object) = Val;
    return ERustealErrorCode::Ok;
}

// ---------------------------------------------------------------------------
// Double
// ---------------------------------------------------------------------------

static ERustealErrorCode GetF64Impl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop, double* Out)
{
    RUSTEAL_CHECK_VALID(Obj);
    FProperty* Property = static_cast<FProperty*>(Prop.ptr);
    *Out = *Property->ContainerPtrToValuePtr<double>(Object);
    return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetF64Impl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop, double Val)
{
    RUSTEAL_CHECK_VALID(Obj);
    FProperty* Property = static_cast<FProperty*>(Prop.ptr);
    *Property->ContainerPtrToValuePtr<double>(Object) = Val;
    return ERustealErrorCode::Ok;
}

// ---------------------------------------------------------------------------
// String (handles FStrProperty and FTextProperty)
// ---------------------------------------------------------------------------

static ERustealErrorCode GetStringImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop,
                                     uint8* Buf, uint32 BufLen, uint32* OutLen)
{
    RUSTEAL_CHECK_VALID(Obj);
    FProperty* Property = static_cast<FProperty*>(Prop.ptr);

    FString Value;
    if (const FStrProperty* StrProp = CastField<FStrProperty>(Property))
    {
        Value = StrProp->GetPropertyValue_InContainer(Object);
    }
    else if (const FTextProperty* TextProp = CastField<FTextProperty>(Property))
    {
        Value = TextProp->GetPropertyValue_InContainer(Object).ToString();
    }
    else
    {
        return ERustealErrorCode::TypeMismatch;
    }

    const FTCHARToUTF8 Utf8(*Value);
    const uint32 Len = static_cast<uint32>(Utf8.Length());

    if (OutLen)
    {
        *OutLen = Len;
    }
    if (Buf && BufLen > 0)
    {
        const uint32 CopyLen = FMath::Min(Len, BufLen);
        FMemory::Memcpy(Buf, Utf8.Get(), CopyLen);
    }
    return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetStringImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop,
                                     const uint8* InBuf, uint32 Len)
{
    RUSTEAL_CHECK_VALID(Obj);
    FProperty* Property = static_cast<FProperty*>(Prop.ptr);

    const FString Value(Len, UTF8_TO_TCHAR(reinterpret_cast<const char*>(InBuf)));

    if (FStrProperty* StrProp = CastField<FStrProperty>(Property))
    {
        StrProp->SetPropertyValue_InContainer(Object, Value);
    }
    else if (FTextProperty* TextProp = CastField<FTextProperty>(Property))
    {
        TextProp->SetPropertyValue_InContainer(Object, FText::FromString(Value));
    }
    else
    {
        return ERustealErrorCode::TypeMismatch;
    }
    return ERustealErrorCode::Ok;
}

// ---------------------------------------------------------------------------
// FName (stored as opaque uint64)
// ---------------------------------------------------------------------------

static ERustealErrorCode GetFNameImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop, RustealFNameHandle* Out)
{
    RUSTEAL_CHECK_VALID(Obj);
    FProperty* Property = static_cast<FProperty*>(Prop.ptr);
    const FName* NamePtr = Property->ContainerPtrToValuePtr<FName>(Object);
    static_assert(sizeof(FName) >= sizeof(uint64), "FName must be at least 8 bytes");
    FMemory::Memcpy(&Out->value, NamePtr, sizeof(uint64));
    return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetFNameImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop, RustealFNameHandle Val)
{
    RUSTEAL_CHECK_VALID(Obj);
    FProperty* Property = static_cast<FProperty*>(Prop.ptr);
    FName* NamePtr = Property->ContainerPtrToValuePtr<FName>(Object);
    FMemory::Memcpy(NamePtr, &Val.value, sizeof(uint64));
    return ERustealErrorCode::Ok;
}

// ---------------------------------------------------------------------------
// Object reference (covers ObjectProperty, ClassProperty, TObjectPtr)
// ---------------------------------------------------------------------------

static ERustealErrorCode GetObjectImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop, RustealUObjectHandle* Out)
{
    RUSTEAL_CHECK_VALID(Obj);
    const FObjectPropertyBase* ObjProp = CastField<FObjectPropertyBase>(static_cast<FProperty*>(Prop.ptr));
    if (!ObjProp)
    {
        return ERustealErrorCode::TypeMismatch;
    }
    UObject* Value = ObjProp->GetObjectPropertyValue_InContainer(Object);
    Out->ptr = Value;
    return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetObjectImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop, RustealUObjectHandle Val)
{
    RUSTEAL_CHECK_VALID(Obj);
    FObjectPropertyBase* ObjProp = CastField<FObjectPropertyBase>(static_cast<FProperty*>(Prop.ptr));
    if (!ObjProp)
    {
        return ERustealErrorCode::TypeMismatch;
    }
    ObjProp->SetObjectPropertyValue_InContainer(Object, static_cast<UObject*>(Val.ptr));
    return ERustealErrorCode::Ok;
}

// ---------------------------------------------------------------------------
// Enum (handles FEnumProperty and enum-backed FByteProperty)
// ---------------------------------------------------------------------------

static ERustealErrorCode GetEnumImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop, int64* Out)
{
    RUSTEAL_CHECK_VALID(Obj);
    FProperty* Property = static_cast<FProperty*>(Prop.ptr);

    if (const FEnumProperty* EnumProp = CastField<FEnumProperty>(Property))
    {
        const FNumericProperty* UnderlyingProp = EnumProp->GetUnderlyingProperty();
        const void* ValuePtr = EnumProp->ContainerPtrToValuePtr<void>(Object);
        *Out = UnderlyingProp->GetSignedIntPropertyValue(ValuePtr);
    }
    else if (const FByteProperty* ByteProp = CastField<FByteProperty>(Property))
    {
        *Out = static_cast<int64>(*ByteProp->ContainerPtrToValuePtr<uint8>(Object));
    }
    else
    {
        return ERustealErrorCode::TypeMismatch;
    }
    return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetEnumImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop, int64 Val)
{
    RUSTEAL_CHECK_VALID(Obj);
    FProperty* Property = static_cast<FProperty*>(Prop.ptr);

    if (const FEnumProperty* EnumProp = CastField<FEnumProperty>(Property))
    {
        FNumericProperty* UnderlyingProp = const_cast<FNumericProperty*>(EnumProp->GetUnderlyingProperty());
        void* ValuePtr = EnumProp->ContainerPtrToValuePtr<void>(Object);
        UnderlyingProp->SetIntPropertyValue(ValuePtr, Val);
    }
    else if (FByteProperty* ByteProp = CastField<FByteProperty>(Property))
    {
        *ByteProp->ContainerPtrToValuePtr<uint8>(Object) = static_cast<uint8>(Val);
    }
    else
    {
        return ERustealErrorCode::TypeMismatch;
    }
    return ERustealErrorCode::Ok;
}

// ---------------------------------------------------------------------------
// Struct (raw memory copy via FStructProperty)
// ---------------------------------------------------------------------------

static ERustealErrorCode GetStructImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop,
                                     uint8* OutBuf, uint32 BufSize)
{
    RUSTEAL_CHECK_VALID(Obj);
    const FStructProperty* StructProp = CastField<FStructProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!StructProp)
    {
        return ERustealErrorCode::TypeMismatch;
    }
    const void* SrcPtr = StructProp->ContainerPtrToValuePtr<void>(Object);
    StructProp->Struct->CopyScriptStruct(OutBuf, SrcPtr);
    return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetStructImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop,
                                     const uint8* InBuf, uint32 BufSize)
{
    RUSTEAL_CHECK_VALID(Obj);
    const FStructProperty* StructProp = CastField<FStructProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!StructProp)
    {
        return ERustealErrorCode::TypeMismatch;
    }
    void* DstPtr = StructProp->ContainerPtrToValuePtr<void>(Object);
    StructProp->Struct->CopyScriptStruct(DstPtr, InBuf);
    return ERustealErrorCode::Ok;
}

// ---------------------------------------------------------------------------
// Indexed property access (fixed arrays with array_dim > 1)
// ---------------------------------------------------------------------------

static ERustealErrorCode GetPropertyAtImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop,
    uint32 Index, uint8* OutBuf, uint32 BufSize)
{
    RUSTEAL_CHECK_VALID(Obj);
    FProperty* Property = static_cast<FProperty*>(Prop.ptr);
    if (!Property) return ERustealErrorCode::PropertyNotFound;
    if (Index >= (uint32)Property->ArrayDim) return ERustealErrorCode::IndexOutOfRange;

    const void* Src = Property->ContainerPtrToValuePtr<void>(Object, Index);
    uint32 ElemSize = Property->GetElementSize();
    if (BufSize < ElemSize) return ERustealErrorCode::InternalError;

    Property->CopySingleValue(OutBuf, Src);
    return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetPropertyAtImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop,
    uint32 Index, const uint8* InBuf, uint32 BufSize)
{
    RUSTEAL_CHECK_VALID(Obj);
    FProperty* Property = static_cast<FProperty*>(Prop.ptr);
    if (!Property) return ERustealErrorCode::PropertyNotFound;
    if (Index >= (uint32)Property->ArrayDim) return ERustealErrorCode::IndexOutOfRange;

    void* Dest = Property->ContainerPtrToValuePtr<void>(Object, Index);
    uint32 ElemSize = Property->GetElementSize();
    if (BufSize < ElemSize) return ERustealErrorCode::InternalError;

    Property->CopySingleValue(Dest, InBuf);
    return ERustealErrorCode::Ok;
}

// ---------------------------------------------------------------------------
// Static instance
// ---------------------------------------------------------------------------

FRustealPropertyApi GPropertyApi = {
    // Bool
    &GetBoolImpl,
    &SetBoolImpl,
    // Int32
    &GetI32Impl,
    &SetI32Impl,
    // Int64
    &GetI64Impl,
    &SetI64Impl,
    // UInt8
    &GetU8Impl,
    &SetU8Impl,
    // Float
    &GetF32Impl,
    &SetF32Impl,
    // Double
    &GetF64Impl,
    &SetF64Impl,
    // String
    &GetStringImpl,
    &SetStringImpl,
    // FName
    &GetFNameImpl,
    &SetFNameImpl,
    // Object
    &GetObjectImpl,
    &SetObjectImpl,
    // Enum
    &GetEnumImpl,
    &SetEnumImpl,
    // Struct
    &GetStructImpl,
    &SetStructImpl,
    // Indexed access (fixed arrays)
    &GetPropertyAtImpl,
    &SetPropertyAtImpl,
};
