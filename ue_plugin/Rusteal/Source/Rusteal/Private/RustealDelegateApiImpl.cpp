#include "RustealApiTable.h"
#include "RustealDelegateProxy.h"
#include "RustealLibrary.h"
#include "RustealFNameHelper.h"
#include "UObject/TextProperty.h"
#include "UObject/UnrealType.h"

#define RUSTEAL_CHECK_ARGS(ObjHandle, PropHandle)                              \
  UObject *Object = static_cast<UObject *>((ObjHandle).ptr);                   \
  if (!Object || !IsValid(Object)) {                                           \
    return ERustealErrorCode::ObjectDestroyed;                                 \
  }                                                                            \
  FProperty *RawProp = static_cast<FProperty *>((PropHandle).ptr);             \
  if (!RawProp) {                                                              \
    return ERustealErrorCode::PropertyNotFound;                                \
  }

static ERustealErrorCode
RustealDelegateApi_BindDelegate(RustealUObjectHandle ObjHandle,
                                RustealFPropertyHandle PropHandle,
                                uint64 CallbackId) {
  RUSTEAL_CHECK_ARGS(ObjHandle, PropHandle);

  FDelegateProperty *DelegateProp = CastField<FDelegateProperty>(RawProp);

  if (!DelegateProp) {
    return ERustealErrorCode::TypeMismatch;
  }

  FScriptDelegate *Delegate =
      DelegateProp->GetPropertyValuePtr_InContainer(Object);

  if (!Delegate) {
    return ERustealErrorCode::InternalError;
  }

  URustealDelegateProxy *Proxy = NewObject<URustealDelegateProxy>(Object);
  Proxy->CallbackId = CallbackId;
  Proxy->Library = RustealCurrentLibrary();
  Proxy->Signature = DelegateProp->SignatureFunction;
  Proxy->OwnerObject = Object;

  Delegate->BindUFunction(Proxy, URustealDelegateProxy::FakeFuncName);

  return ERustealErrorCode::Ok;
}

static ERustealErrorCode
RustealDelegateApi_UnbindDelegate(RustealUObjectHandle ObjHandle,
                                  RustealFPropertyHandle PropHandle) {
  RUSTEAL_CHECK_ARGS(ObjHandle, PropHandle);

  FDelegateProperty *DelegateProp = CastField<FDelegateProperty>(RawProp);

  if (!DelegateProp) {
    return ERustealErrorCode::TypeMismatch;
  }

  FScriptDelegate *Delegate =
      DelegateProp->GetPropertyValuePtr_InContainer(Object);

  if (!Delegate) {
    return ERustealErrorCode::InternalError;
  }

  Delegate->Unbind();
  return ERustealErrorCode::Ok;
}

static ERustealErrorCode
RustealDelegateApi_AddMulticast(RustealUObjectHandle ObjHandle,
                                RustealFPropertyHandle PropHandle,
                                uint64 CallbackId) {
  RUSTEAL_CHECK_ARGS(ObjHandle, PropHandle);

  FMulticastDelegateProperty *MultiProp =
      CastField<FMulticastDelegateProperty>(RawProp);

  if (!MultiProp) {
    return ERustealErrorCode::TypeMismatch;
  }

  URustealDelegateProxy *Proxy = NewObject<URustealDelegateProxy>(Object);
  Proxy->CallbackId = CallbackId;
  Proxy->Library = RustealCurrentLibrary();
  Proxy->Signature = MultiProp->SignatureFunction;
  Proxy->OwnerObject = Object;

  FScriptDelegate ScriptDelegate;
  ScriptDelegate.BindUFunction(Proxy, URustealDelegateProxy::FakeFuncName);

  MultiProp->AddDelegate(MoveTemp(ScriptDelegate), Object);

  return ERustealErrorCode::Ok;
}

static ERustealErrorCode
RustealDelegateApi_RemoveMulticast(RustealUObjectHandle ObjHandle,
                                   RustealFPropertyHandle PropHandle,
                                   uint64 CallbackId) {
  RUSTEAL_CHECK_ARGS(ObjHandle, PropHandle);

  FMulticastDelegateProperty *MultiProp =
      CastField<FMulticastDelegateProperty>(RawProp);

  if (!MultiProp) {
    return ERustealErrorCode::TypeMismatch;
  }

  TArray<UObject *> Children;
  GetObjectsWithOuter(Object, Children, false);

  for (UObject *Child : Children) {
    URustealDelegateProxy *Proxy = Cast<URustealDelegateProxy>(Child);

    if (Proxy && Proxy->CallbackId == CallbackId &&
        Proxy->Library == RustealCurrentLibrary()) {
      FScriptDelegate ScriptDelegate;
      ScriptDelegate.BindUFunction(Proxy, URustealDelegateProxy::FakeFuncName);
      MultiProp->RemoveDelegate(ScriptDelegate, Object);
      return ERustealErrorCode::Ok;
    }
  }

  return ERustealErrorCode::Ok;
}

static ERustealErrorCode
RustealDelegateApi_BroadcastMulticast(RustealUObjectHandle ObjHandle,
                                      RustealFPropertyHandle PropHandle,
                                      uint8 *Params) {
  RUSTEAL_CHECK_ARGS(ObjHandle, PropHandle);

  FMulticastDelegateProperty *MultiProp =
      CastField<FMulticastDelegateProperty>(RawProp);

  if (!MultiProp) {
    return ERustealErrorCode::TypeMismatch;
  }

  const FMulticastScriptDelegate *Delegate = MultiProp->GetMulticastDelegate(
      MultiProp->ContainerPtrToValuePtr<void>(Object));

  if (Delegate) {
    Delegate->ProcessDelegate<UObject>(Params);
  }

  return ERustealErrorCode::Ok;
}

static ERustealErrorCode
RustealDelegateApi_ReadParam(RustealFPropertyHandle PropHandle, void *ParamsBuf,
                             uint32 Offset, uint8 *OutBuf, uint32 OutBufSize,
                             uint32 *OutWritten) {
  FProperty *Prop = static_cast<FProperty *>(PropHandle.ptr);

  if (!Prop || !ParamsBuf) {
    return ERustealErrorCode::NullArgument;
  }

  const void *ValuePtr = static_cast<const uint8 *>(ParamsBuf) + Offset;

  if (const FStrProperty *StrProp = CastField<FStrProperty>(Prop)) {
    const FString &Str = StrProp->GetPropertyValue(ValuePtr);
    FTCHARToUTF8 Utf8(*Str);
    uint32 Len = static_cast<uint32>(Utf8.Length());
    uint32 Required = sizeof(uint32) + Len;

    if (OutWritten)
      *OutWritten = Required;

    if (OutBufSize < Required) {
      return ERustealErrorCode::BufferTooSmall;
    }

    FMemory::Memcpy(OutBuf, &Len, sizeof(uint32));
    FMemory::Memcpy(OutBuf + sizeof(uint32), Utf8.Get(), Len);
    return ERustealErrorCode::Ok;
  }

  if (const FTextProperty *TextProp = CastField<FTextProperty>(Prop)) {
    FString Str = TextProp->GetPropertyValue(ValuePtr).ToString();
    FTCHARToUTF8 Utf8(*Str);
    uint32 Len = static_cast<uint32>(Utf8.Length());
    uint32 Required = sizeof(uint32) + Len;

    if (OutWritten)
      *OutWritten = Required;

    if (OutBufSize < Required) {
      return ERustealErrorCode::BufferTooSmall;
    }

    FMemory::Memcpy(OutBuf, &Len, sizeof(uint32));
    FMemory::Memcpy(OutBuf + sizeof(uint32), Utf8.Get(), Len);
    return ERustealErrorCode::Ok;
  }

  if (CastField<FNameProperty>(Prop)) {
    const FName *NamePtr = static_cast<const FName *>(ValuePtr);
    uint64 Packed = RustealPackFName(*NamePtr);

    if (OutWritten)
      *OutWritten = sizeof(uint64);

    if (OutBufSize < sizeof(uint64)) {
      return ERustealErrorCode::BufferTooSmall;
    }

    FMemory::Memcpy(OutBuf, &Packed, sizeof(uint64));
    return ERustealErrorCode::Ok;
  }

  if (const FStructProperty *StructProp = CastField<FStructProperty>(Prop)) {
    uint32 Size = StructProp->GetSize();

    if (OutWritten)
      *OutWritten = Size;

    if (OutBufSize < Size) {
      return ERustealErrorCode::BufferTooSmall;
    }

    StructProp->Struct->CopyScriptStruct(OutBuf, ValuePtr);
    return ERustealErrorCode::Ok;
  }

  if (const FObjectPropertyBase *ObjProp =
          CastField<FObjectPropertyBase>(Prop)) {
    UObject *Obj = ObjProp->GetObjectPropertyValue(ValuePtr);

    if (OutWritten)
      *OutWritten = sizeof(void *);

    if (OutBufSize < sizeof(void *)) {
      return ERustealErrorCode::BufferTooSmall;
    }

    FMemory::Memcpy(OutBuf, &Obj, sizeof(void *));
    return ERustealErrorCode::Ok;
  }

  uint32 Size = Prop->GetSize();

  if (OutWritten)
    *OutWritten = Size;

  if (OutBufSize < Size) {
    return ERustealErrorCode::BufferTooSmall;
  }

  FMemory::Memcpy(OutBuf, ValuePtr, Size);
  return ERustealErrorCode::Ok;
}

static ERustealErrorCode RustealDelegateApi_AddFunction(
    RustealUObjectHandle ObjHandle, RustealFPropertyHandle PropHandle,
    RustealUObjectHandle TargetHandle, const uint8 *Name, uint32 NameLen) {
  RUSTEAL_CHECK_ARGS(ObjHandle, PropHandle);

  UObject *Target = static_cast<UObject *>(TargetHandle.ptr);

  if (!IsValid(Target)) {
    return ERustealErrorCode::ObjectDestroyed;
  }

  const FUTF8ToTCHAR NameChars(reinterpret_cast<const ANSICHAR *>(Name),
                               NameLen);

  const FName FunctionName(NameChars.Length(), NameChars.Get());
  UFunction *Function = Target->FindFunction(FunctionName);

  if (!Function) {
    return ERustealErrorCode::FunctionNotFound;
  }

  FScriptDelegate ScriptDelegate;
  ScriptDelegate.BindUFunction(Target, FunctionName);

  const uint64 IgnoredFlags =
      UFunction::GetDefaultIgnoredSignatureCompatibilityFlags() | CPF_OutParm |
      CPF_ReferenceParm;

  if (FMulticastDelegateProperty *MultiProp =
          CastField<FMulticastDelegateProperty>(RawProp)) {
    if (!Function->IsSignatureCompatibleWith(MultiProp->SignatureFunction,
                                             IgnoredFlags)) {
      return ERustealErrorCode::TypeMismatch;
    }

    MultiProp->AddDelegate(MoveTemp(ScriptDelegate), Object);
    return ERustealErrorCode::Ok;
  }

  if (FDelegateProperty *DelegateProp = CastField<FDelegateProperty>(RawProp)) {
    if (!Function->IsSignatureCompatibleWith(DelegateProp->SignatureFunction,
                                             IgnoredFlags)) {
      return ERustealErrorCode::TypeMismatch;
    }

    *DelegateProp->GetPropertyValuePtr_InContainer(Object) = ScriptDelegate;
    return ERustealErrorCode::Ok;
  }

  return ERustealErrorCode::TypeMismatch;
}

FRustealDelegateApi GDelegateApi = {
    &RustealDelegateApi_BindDelegate,       &RustealDelegateApi_UnbindDelegate,
    &RustealDelegateApi_AddMulticast,       &RustealDelegateApi_RemoveMulticast,
    &RustealDelegateApi_BroadcastMulticast, &RustealDelegateApi_ReadParam,
    &RustealDelegateApi_AddFunction,
};
