// RustealDelegateApiImpl.cpp — FRustealDelegateApi implementation.
// Bridges Rust closures to UE delegates (unicast and multicast).

#include "RustealApiTable.h"
#include "RustealDelegateProxy.h"
#include "RustealFNameHelper.h"
#include "UObject/UnrealType.h"
#include "UObject/TextProperty.h"

#include "RustealModule.h"

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#define RUSTEAL_CHECK_ARGS(ObjHandle, PropHandle)                              \
    UObject* Object = static_cast<UObject*>((ObjHandle).ptr);               \
    if (!Object || !IsValid(Object))                                        \
    {                                                                       \
        return ERustealErrorCode::ObjectDestroyed;                             \
    }                                                                       \
    FProperty* RawProp = static_cast<FProperty*>((PropHandle).ptr);         \
    if (!RawProp)                                                           \
    {                                                                       \
        return ERustealErrorCode::PropertyNotFound;                            \
    }

// ---------------------------------------------------------------------------
// bind_delegate — bind a Rust callback to a unicast delegate
// ---------------------------------------------------------------------------

static ERustealErrorCode RustealDelegateApi_BindDelegate(
    RustealUObjectHandle ObjHandle,
    RustealFPropertyHandle PropHandle,
    uint64 CallbackId)
{
    RUSTEAL_CHECK_ARGS(ObjHandle, PropHandle);

    FDelegateProperty* DelegateProp = CastField<FDelegateProperty>(RawProp);
    if (!DelegateProp)
    {
        return ERustealErrorCode::TypeMismatch;
    }

    FScriptDelegate* Delegate = DelegateProp->GetPropertyValuePtr_InContainer(Object);
    if (!Delegate)
    {
        return ERustealErrorCode::InternalError;
    }

    // Create the proxy with Object as outer (lifecycle tied to owner).
    URustealDelegateProxy* Proxy = NewObject<URustealDelegateProxy>(Object);
    Proxy->CallbackId = CallbackId;
    Proxy->Signature = DelegateProp->SignatureFunction;
    Proxy->OwnerObject = Object;

    Delegate->BindUFunction(Proxy, URustealDelegateProxy::FakeFuncName);

    return ERustealErrorCode::Ok;
}

// ---------------------------------------------------------------------------
// unbind_delegate — unbind a unicast delegate
// ---------------------------------------------------------------------------

static ERustealErrorCode RustealDelegateApi_UnbindDelegate(
    RustealUObjectHandle ObjHandle,
    RustealFPropertyHandle PropHandle)
{
    RUSTEAL_CHECK_ARGS(ObjHandle, PropHandle);

    FDelegateProperty* DelegateProp = CastField<FDelegateProperty>(RawProp);
    if (!DelegateProp)
    {
        return ERustealErrorCode::TypeMismatch;
    }

    FScriptDelegate* Delegate = DelegateProp->GetPropertyValuePtr_InContainer(Object);
    if (!Delegate)
    {
        return ERustealErrorCode::InternalError;
    }

    Delegate->Unbind();
    return ERustealErrorCode::Ok;
}

// ---------------------------------------------------------------------------
// add_multicast — add a Rust callback to a multicast delegate
// ---------------------------------------------------------------------------

static ERustealErrorCode RustealDelegateApi_AddMulticast(
    RustealUObjectHandle ObjHandle,
    RustealFPropertyHandle PropHandle,
    uint64 CallbackId)
{
    RUSTEAL_CHECK_ARGS(ObjHandle, PropHandle);

    FMulticastDelegateProperty* MultiProp = CastField<FMulticastDelegateProperty>(RawProp);
    if (!MultiProp)
    {
        return ERustealErrorCode::TypeMismatch;
    }

    // Create the proxy with Object as outer.
    URustealDelegateProxy* Proxy = NewObject<URustealDelegateProxy>(Object);
    Proxy->CallbackId = CallbackId;
    Proxy->Signature = MultiProp->SignatureFunction;
    Proxy->OwnerObject = Object;

    // Build a script delegate targeting the proxy.
    FScriptDelegate ScriptDelegate;
    ScriptDelegate.BindUFunction(Proxy, URustealDelegateProxy::FakeFuncName);

    // AddDelegate works for both Inline and Sparse multicast delegates.
    MultiProp->AddDelegate(MoveTemp(ScriptDelegate), Object);

    return ERustealErrorCode::Ok;
}

// ---------------------------------------------------------------------------
// remove_multicast — remove a Rust callback from a multicast delegate
// ---------------------------------------------------------------------------

static ERustealErrorCode RustealDelegateApi_RemoveMulticast(
    RustealUObjectHandle ObjHandle,
    RustealFPropertyHandle PropHandle,
    uint64 CallbackId)
{
    RUSTEAL_CHECK_ARGS(ObjHandle, PropHandle);

    FMulticastDelegateProperty* MultiProp = CastField<FMulticastDelegateProperty>(RawProp);
    if (!MultiProp)
    {
        return ERustealErrorCode::TypeMismatch;
    }

    // Get the invocation list to find our proxy by CallbackId.
    // For inline delegates we can access the invocation list directly.
    // For both types, we iterate to find the matching proxy.
    // Iterate all URustealDelegateProxy objects owned by this Object to find the matching one.
    TArray<UObject*> Children;
    GetObjectsWithOuter(Object, Children, false);
    for (UObject* Child : Children)
    {
        URustealDelegateProxy* Proxy = Cast<URustealDelegateProxy>(Child);
        if (Proxy && Proxy->CallbackId == CallbackId)
        {
            FScriptDelegate ScriptDelegate;
            ScriptDelegate.BindUFunction(Proxy, URustealDelegateProxy::FakeFuncName);
            MultiProp->RemoveDelegate(ScriptDelegate, Object);
            return ERustealErrorCode::Ok;
        }
    }

    // CallbackId not found — not an error, just means it wasn't bound.
    return ERustealErrorCode::Ok;
}

// ---------------------------------------------------------------------------
// broadcast_multicast — trigger a multicast delegate from Rust
// ---------------------------------------------------------------------------

static ERustealErrorCode RustealDelegateApi_BroadcastMulticast(
    RustealUObjectHandle ObjHandle,
    RustealFPropertyHandle PropHandle,
    uint8* Params)
{
    RUSTEAL_CHECK_ARGS(ObjHandle, PropHandle);

    FMulticastDelegateProperty* MultiProp = CastField<FMulticastDelegateProperty>(RawProp);
    if (!MultiProp)
    {
        return ERustealErrorCode::TypeMismatch;
    }

    // Use the multicast delegate's built-in broadcast mechanism.
    // This calls ProcessMulticastDelegate which fires all bound delegates.
    // ProcessMulticastDelegate is the ProcessEvent-based broadcast path.
    Object->ProcessEvent(MultiProp->SignatureFunction, Params);

    return ERustealErrorCode::Ok;
}

// ---------------------------------------------------------------------------
// read_param — read a typed parameter from a raw ProcessEvent params buffer
// ---------------------------------------------------------------------------

static ERustealErrorCode RustealDelegateApi_ReadParam(
    RustealFPropertyHandle PropHandle,
    void* ParamsBuf,
    uint32 Offset,
    uint8* OutBuf,
    uint32 OutBufSize,
    uint32* OutWritten)
{
    FProperty* Prop = static_cast<FProperty*>(PropHandle.ptr);
    if (!Prop || !ParamsBuf)
    {
        return ERustealErrorCode::NullArgument;
    }

    const void* ValuePtr = static_cast<const uint8*>(ParamsBuf) + Offset;

    // String (FString)
    if (const FStrProperty* StrProp = CastField<FStrProperty>(Prop))
    {
        const FString& Str = StrProp->GetPropertyValue(ValuePtr);
        FTCHARToUTF8 Utf8(*Str);
        uint32 Len = static_cast<uint32>(Utf8.Length());
        uint32 Required = sizeof(uint32) + Len;
        if (OutWritten) *OutWritten = Required;
        if (OutBufSize < Required)
        {
            return ERustealErrorCode::BufferTooSmall;
        }
        FMemory::Memcpy(OutBuf, &Len, sizeof(uint32));
        FMemory::Memcpy(OutBuf + sizeof(uint32), Utf8.Get(), Len);
        return ERustealErrorCode::Ok;
    }

    // Text (FText -> FString)
    if (const FTextProperty* TextProp = CastField<FTextProperty>(Prop))
    {
        FString Str = TextProp->GetPropertyValue(ValuePtr).ToString();
        FTCHARToUTF8 Utf8(*Str);
        uint32 Len = static_cast<uint32>(Utf8.Length());
        uint32 Required = sizeof(uint32) + Len;
        if (OutWritten) *OutWritten = Required;
        if (OutBufSize < Required)
        {
            return ERustealErrorCode::BufferTooSmall;
        }
        FMemory::Memcpy(OutBuf, &Len, sizeof(uint32));
        FMemory::Memcpy(OutBuf + sizeof(uint32), Utf8.Get(), Len);
        return ERustealErrorCode::Ok;
    }

    // FName -> pack into uint64
    if (CastField<FNameProperty>(Prop))
    {
        const FName* NamePtr = static_cast<const FName*>(ValuePtr);
        uint64 Packed = RustealPackFName(*NamePtr);
        if (OutWritten) *OutWritten = sizeof(uint64);
        if (OutBufSize < sizeof(uint64))
        {
            return ERustealErrorCode::BufferTooSmall;
        }
        FMemory::Memcpy(OutBuf, &Packed, sizeof(uint64));
        return ERustealErrorCode::Ok;
    }

    // Struct -> CopyScriptStruct
    if (const FStructProperty* StructProp = CastField<FStructProperty>(Prop))
    {
        uint32 Size = StructProp->GetSize();
        if (OutWritten) *OutWritten = Size;
        if (OutBufSize < Size)
        {
            return ERustealErrorCode::BufferTooSmall;
        }
        StructProp->Struct->CopyScriptStruct(OutBuf, ValuePtr);
        return ERustealErrorCode::Ok;
    }

    // Object
    if (const FObjectPropertyBase* ObjProp = CastField<FObjectPropertyBase>(Prop))
    {
        UObject* Obj = ObjProp->GetObjectPropertyValue(ValuePtr);
        if (OutWritten) *OutWritten = sizeof(void*);
        if (OutBufSize < sizeof(void*))
        {
            return ERustealErrorCode::BufferTooSmall;
        }
        FMemory::Memcpy(OutBuf, &Obj, sizeof(void*));
        return ERustealErrorCode::Ok;
    }

    // Fallback: raw memcpy (primitives, enums)
    uint32 Size = Prop->GetSize();
    if (OutWritten) *OutWritten = Size;
    if (OutBufSize < Size)
    {
        return ERustealErrorCode::BufferTooSmall;
    }
    FMemory::Memcpy(OutBuf, ValuePtr, Size);
    return ERustealErrorCode::Ok;
}

// ---------------------------------------------------------------------------
// Global API struct
// ---------------------------------------------------------------------------

FRustealDelegateApi GDelegateApi = {
    &RustealDelegateApi_BindDelegate,
    &RustealDelegateApi_UnbindDelegate,
    &RustealDelegateApi_AddMulticast,
    &RustealDelegateApi_RemoveMulticast,
    &RustealDelegateApi_BroadcastMulticast,
    &RustealDelegateApi_ReadParam,
};
