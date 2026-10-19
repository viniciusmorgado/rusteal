#include "RustealApiTable.h"
#include "RustealErrors.h"
#include "RustealFNameHelper.h"
#include "UObject/TextProperty.h"
#include "UObject/UObjectGlobals.h"
#include "UObject/UnrealType.h"
#include "RustealStrings.h"
#include "RustealUtf8.h"

#define RUSTEAL_CHECK_VALID(ObjHandle)                                         \
  void *Object = (ObjHandle).ptr;                                              \
  if (!Object) {                                                               \
    return RustealFail(ERustealErrorCode::ObjectDestroyed);                    \
  }

static ERustealErrorCode GetBoolImpl(RustealUObjectHandle Obj,
                                     RustealFPropertyHandle Prop, bool *Out) {
  RUSTEAL_CHECK_VALID(Obj);

  const FBoolProperty *BoolProp =
      CastField<FBoolProperty>(static_cast<FProperty *>(Prop.ptr));

  if (!BoolProp) {
    return RustealFailType(Prop.ptr, TEXT("FBoolProperty"));
  }

  *Out = BoolProp->GetPropertyValue_InContainer(Object);
  return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetBoolImpl(RustealUObjectHandle Obj,
                                     RustealFPropertyHandle Prop, bool Val) {
  RUSTEAL_CHECK_VALID(Obj);

  FBoolProperty *BoolProp =
      CastField<FBoolProperty>(static_cast<FProperty *>(Prop.ptr));

  if (!BoolProp) {
    return RustealFailType(Prop.ptr, TEXT("FBoolProperty"));
  }

  BoolProp->SetPropertyValue_InContainer(Object, Val);
  return ERustealErrorCode::Ok;
}

static ERustealErrorCode GetI32Impl(RustealUObjectHandle Obj,
                                    RustealFPropertyHandle Prop, int32 *Out) {
  RUSTEAL_CHECK_VALID(Obj);
  FProperty *Property = static_cast<FProperty *>(Prop.ptr);
  *Out = *Property->ContainerPtrToValuePtr<int32>(Object);
  return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetI32Impl(RustealUObjectHandle Obj,
                                    RustealFPropertyHandle Prop, int32 Val) {
  RUSTEAL_CHECK_VALID(Obj);
  FProperty *Property = static_cast<FProperty *>(Prop.ptr);
  *Property->ContainerPtrToValuePtr<int32>(Object) = Val;
  return ERustealErrorCode::Ok;
}

static ERustealErrorCode GetI64Impl(RustealUObjectHandle Obj,
                                    RustealFPropertyHandle Prop, int64 *Out) {
  RUSTEAL_CHECK_VALID(Obj);
  FProperty *Property = static_cast<FProperty *>(Prop.ptr);
  *Out = *Property->ContainerPtrToValuePtr<int64>(Object);
  return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetI64Impl(RustealUObjectHandle Obj,
                                    RustealFPropertyHandle Prop, int64 Val) {
  RUSTEAL_CHECK_VALID(Obj);
  FProperty *Property = static_cast<FProperty *>(Prop.ptr);
  *Property->ContainerPtrToValuePtr<int64>(Object) = Val;
  return ERustealErrorCode::Ok;
}

static ERustealErrorCode GetU8Impl(RustealUObjectHandle Obj,
                                   RustealFPropertyHandle Prop, uint8 *Out) {
  RUSTEAL_CHECK_VALID(Obj);
  FProperty *Property = static_cast<FProperty *>(Prop.ptr);
  *Out = *Property->ContainerPtrToValuePtr<uint8>(Object);
  return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetU8Impl(RustealUObjectHandle Obj,
                                   RustealFPropertyHandle Prop, uint8 Val) {
  RUSTEAL_CHECK_VALID(Obj);
  FProperty *Property = static_cast<FProperty *>(Prop.ptr);
  *Property->ContainerPtrToValuePtr<uint8>(Object) = Val;
  return ERustealErrorCode::Ok;
}

static ERustealErrorCode GetF32Impl(RustealUObjectHandle Obj,
                                    RustealFPropertyHandle Prop, float *Out) {
  RUSTEAL_CHECK_VALID(Obj);
  FProperty *Property = static_cast<FProperty *>(Prop.ptr);
  *Out = *Property->ContainerPtrToValuePtr<float>(Object);
  return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetF32Impl(RustealUObjectHandle Obj,
                                    RustealFPropertyHandle Prop, float Val) {
  RUSTEAL_CHECK_VALID(Obj);
  FProperty *Property = static_cast<FProperty *>(Prop.ptr);
  *Property->ContainerPtrToValuePtr<float>(Object) = Val;
  return ERustealErrorCode::Ok;
}

static ERustealErrorCode GetF64Impl(RustealUObjectHandle Obj,
                                    RustealFPropertyHandle Prop, double *Out) {
  RUSTEAL_CHECK_VALID(Obj);
  FProperty *Property = static_cast<FProperty *>(Prop.ptr);
  *Out = *Property->ContainerPtrToValuePtr<double>(Object);
  return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetF64Impl(RustealUObjectHandle Obj,
                                    RustealFPropertyHandle Prop, double Val) {
  RUSTEAL_CHECK_VALID(Obj);
  FProperty *Property = static_cast<FProperty *>(Prop.ptr);
  *Property->ContainerPtrToValuePtr<double>(Object) = Val;
  return ERustealErrorCode::Ok;
}

static ERustealErrorCode GetStringImpl(RustealUObjectHandle Obj,
                                       RustealFPropertyHandle Prop, uint8 *Buf,
                                       uint32 BufLen, uint32 *OutLen) {
  RUSTEAL_CHECK_VALID(Obj);
  FProperty *Property = static_cast<FProperty *>(Prop.ptr);

  FString Value;

  if (const FStrProperty *StrProp = CastField<FStrProperty>(Property)) {
    Value = StrProp->GetPropertyValue_InContainer(Object);
  } else if (const FTextProperty *TextProp =
                 CastField<FTextProperty>(Property)) {
    Value = TextProp->GetPropertyValue_InContainer(Object).ToString();
  } else {
    return RustealFailType(Property, TEXT("FTextProperty"));
  }

  RustealWriteUtf8(Value, Buf, BufLen, OutLen);
  return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetStringImpl(RustealUObjectHandle Obj,
                                       RustealFPropertyHandle Prop,
                                       const uint8 *InBuf, uint32 Len) {
  RUSTEAL_CHECK_VALID(Obj);
  FProperty *Property = static_cast<FProperty *>(Prop.ptr);

  const FString Value = RustealUtf8ToFString(InBuf, Len);

  if (FStrProperty *StrProp = CastField<FStrProperty>(Property)) {
    StrProp->SetPropertyValue_InContainer(Object, Value);
  } else if (FTextProperty *TextProp = CastField<FTextProperty>(Property)) {
    TextProp->SetPropertyValue_InContainer(Object, FText::FromString(Value));
  } else {
    return RustealFailType(Property, TEXT("FTextProperty"));
  }

  return ERustealErrorCode::Ok;
}

static ERustealErrorCode GetSoftObjectPathImpl(RustealUObjectHandle Obj,
                                               RustealFPropertyHandle Prop,
                                               uint8 *Buf, uint32 BufLen,
                                               uint32 *OutLen) {
  RUSTEAL_CHECK_VALID(Obj);

  const FSoftObjectProperty *SoftProp =
      CastField<FSoftObjectProperty>(static_cast<FProperty *>(Prop.ptr));

  if (!SoftProp) {
    return RustealFailType(Prop.ptr, TEXT("FSoftObjectProperty"));
  }

  const FString Value = SoftProp->GetPropertyValue_InContainer(Object)
                            .ToSoftObjectPath()
                            .ToString();

  const FTCHARToUTF8 Utf8(*Value);
  const uint32 Len = static_cast<uint32>(Utf8.Length());

  if (OutLen) {
    *OutLen = Len;
  }

  if (Buf && BufLen > 0) {
    FMemory::Memcpy(Buf, Utf8.Get(), FMath::Min(Len, BufLen));
  }

  return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetSoftObjectPathImpl(RustealUObjectHandle Obj,
                                               RustealFPropertyHandle Prop,
                                               const uint8 *InBuf, uint32 Len) {
  RUSTEAL_CHECK_VALID(Obj);

  FSoftObjectProperty *SoftProp =
      CastField<FSoftObjectProperty>(static_cast<FProperty *>(Prop.ptr));

  if (!SoftProp) {
    return RustealFailType(Prop.ptr, TEXT("FSoftObjectProperty"));
  }

  const FString Path = RustealUtf8ToFString(InBuf, Len);

  SoftProp->SetPropertyValue_InContainer(Object,
                                         FSoftObjectPtr(FSoftObjectPath(Path)));

  return ERustealErrorCode::Ok;
}

static ERustealErrorCode GetTextImpl(RustealUObjectHandle Obj,
                                     RustealFPropertyHandle Prop, void **Out) {
  RUSTEAL_CHECK_VALID(Obj);

  const FTextProperty *TextProp =
      CastField<FTextProperty>(static_cast<FProperty *>(Prop.ptr));

  if (!TextProp) {
    return RustealFailType(Prop.ptr, TEXT("FTextProperty"));
  }

  *Out = new FText(TextProp->GetPropertyValue_InContainer(Object));
  return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetTextImpl(RustealUObjectHandle Obj,
                                     RustealFPropertyHandle Prop,
                                     const void *Text) {
  RUSTEAL_CHECK_VALID(Obj);

  FTextProperty *TextProp =
      CastField<FTextProperty>(static_cast<FProperty *>(Prop.ptr));

  if (!TextProp) {
    return RustealFailType(Prop.ptr, TEXT("FTextProperty"));
  }

  TextProp->SetPropertyValue_InContainer(Object,
                                         *static_cast<const FText *>(Text));
  return ERustealErrorCode::Ok;
}

static ERustealErrorCode GetFNameImpl(RustealUObjectHandle Obj,
                                      RustealFPropertyHandle Prop,
                                      RustealFNameHandle *Out) {
  RUSTEAL_CHECK_VALID(Obj);
  FProperty *Property = static_cast<FProperty *>(Prop.ptr);
  const FName *NamePtr = Property->ContainerPtrToValuePtr<FName>(Object);
  Out->value = RustealPackFName(*NamePtr);
  return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetFNameImpl(RustealUObjectHandle Obj,
                                      RustealFPropertyHandle Prop,
                                      RustealFNameHandle Val) {
  RUSTEAL_CHECK_VALID(Obj);
  FProperty *Property = static_cast<FProperty *>(Prop.ptr);
  FName *NamePtr = Property->ContainerPtrToValuePtr<FName>(Object);
  *NamePtr = RustealUnpackFName(Val.value);
  return ERustealErrorCode::Ok;
}

static ERustealErrorCode GetObjectImpl(RustealUObjectHandle Obj,
                                       RustealFPropertyHandle Prop,
                                       RustealUObjectHandle *Out) {
  RUSTEAL_CHECK_VALID(Obj);

  if (const FInterfaceProperty *IfaceProp =
          CastField<FInterfaceProperty>(static_cast<FProperty *>(Prop.ptr))) {
    Out->ptr =
        IfaceProp->ContainerPtrToValuePtr<FScriptInterface>(Object)->GetObject();
    return ERustealErrorCode::Ok;
  }

  const FObjectPropertyBase *ObjProp =
      CastField<FObjectPropertyBase>(static_cast<FProperty *>(Prop.ptr));

  if (!ObjProp) {
    return RustealFailType(Prop.ptr, TEXT("FObjectPropertyBase"));
  }

  UObject *Value = ObjProp->GetObjectPropertyValue_InContainer(Object);
  Out->ptr = Value;
  return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetObjectImpl(RustealUObjectHandle Obj,
                                       RustealFPropertyHandle Prop,
                                       RustealUObjectHandle Val) {
  RUSTEAL_CHECK_VALID(Obj);

  if (FInterfaceProperty *IfaceProp =
          CastField<FInterfaceProperty>(static_cast<FProperty *>(Prop.ptr))) {
    UObject *Value = static_cast<UObject *>(Val.ptr);
    FScriptInterface *Iface =
        IfaceProp->ContainerPtrToValuePtr<FScriptInterface>(Object);
    Iface->SetObject(Value);
    Iface->SetInterface(
        Value ? Value->GetInterfaceAddress(IfaceProp->InterfaceClass) : nullptr);
    return ERustealErrorCode::Ok;
  }

  FObjectPropertyBase *ObjProp =
      CastField<FObjectPropertyBase>(static_cast<FProperty *>(Prop.ptr));

  if (!ObjProp) {
    return RustealFailType(Prop.ptr, TEXT("FObjectPropertyBase"));
  }

  ObjProp->SetObjectPropertyValue_InContainer(Object,
                                              static_cast<UObject *>(Val.ptr));

  return ERustealErrorCode::Ok;
}

static ERustealErrorCode GetEnumImpl(RustealUObjectHandle Obj,
                                     RustealFPropertyHandle Prop, int64 *Out) {
  RUSTEAL_CHECK_VALID(Obj);
  FProperty *Property = static_cast<FProperty *>(Prop.ptr);

  if (const FEnumProperty *EnumProp = CastField<FEnumProperty>(Property)) {
    const FNumericProperty *UnderlyingProp = EnumProp->GetUnderlyingProperty();
    const void *ValuePtr = EnumProp->ContainerPtrToValuePtr<void>(Object);
    *Out = UnderlyingProp->GetSignedIntPropertyValue(ValuePtr);
  } else if (const FByteProperty *ByteProp =
                 CastField<FByteProperty>(Property)) {
    *Out = static_cast<int64>(*ByteProp->ContainerPtrToValuePtr<uint8>(Object));
  } else {
    return RustealFailType(Property, TEXT("FByteProperty"));
  }

  return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetEnumImpl(RustealUObjectHandle Obj,
                                     RustealFPropertyHandle Prop, int64 Val) {
  RUSTEAL_CHECK_VALID(Obj);
  FProperty *Property = static_cast<FProperty *>(Prop.ptr);

  if (const FEnumProperty *EnumProp = CastField<FEnumProperty>(Property)) {
    FNumericProperty *UnderlyingProp =
        const_cast<FNumericProperty *>(EnumProp->GetUnderlyingProperty());

    void *ValuePtr = EnumProp->ContainerPtrToValuePtr<void>(Object);
    UnderlyingProp->SetIntPropertyValue(ValuePtr, Val);
  } else if (FByteProperty *ByteProp = CastField<FByteProperty>(Property)) {
    *ByteProp->ContainerPtrToValuePtr<uint8>(Object) = static_cast<uint8>(Val);
  } else {
    return RustealFailType(Property, TEXT("FByteProperty"));
  }

  return ERustealErrorCode::Ok;
}

static ERustealErrorCode GetStructImpl(RustealUObjectHandle Obj,
                                       RustealFPropertyHandle Prop,
                                       uint8 *OutBuf, uint32 BufSize) {
  RUSTEAL_CHECK_VALID(Obj);

  const FStructProperty *StructProp =
      CastField<FStructProperty>(static_cast<FProperty *>(Prop.ptr));

  if (!StructProp) {
    return RustealFailType(Prop.ptr, TEXT("FStructProperty"));
  }

  const void *SrcPtr = StructProp->ContainerPtrToValuePtr<void>(Object);
  StructProp->Struct->InitializeStruct(OutBuf);
  StructProp->Struct->CopyScriptStruct(OutBuf, SrcPtr);
  return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetStructImpl(RustealUObjectHandle Obj,
                                       RustealFPropertyHandle Prop,
                                       const uint8 *InBuf, uint32 BufSize) {
  RUSTEAL_CHECK_VALID(Obj);

  const FStructProperty *StructProp =
      CastField<FStructProperty>(static_cast<FProperty *>(Prop.ptr));

  if (!StructProp) {
    return RustealFailType(Prop.ptr, TEXT("FStructProperty"));
  }

  void *DstPtr = StructProp->ContainerPtrToValuePtr<void>(Object);
  StructProp->Struct->CopyScriptStruct(DstPtr, InBuf);
  return ERustealErrorCode::Ok;
}

static ERustealErrorCode GetPropertyAtImpl(RustealUObjectHandle Obj,
                                           RustealFPropertyHandle Prop,
                                           uint32 Index, uint8 *OutBuf,
                                           uint32 BufSize) {
  RUSTEAL_CHECK_VALID(Obj);
  FProperty *Property = static_cast<FProperty *>(Prop.ptr);

  if (!Property)
    return RustealFail(ERustealErrorCode::PropertyNotFound);

  if (Index >= (uint32)Property->ArrayDim)
    return RustealFail(
        ERustealErrorCode::IndexOutOfRange,
        FString::Printf(TEXT("index %u past the %d elements of %s"), Index,
                        Property->ArrayDim, *RustealDescribe(Property)));

  const void *Src = Property->ContainerPtrToValuePtr<void>(Object, Index);
  uint32 ElemSize = Property->GetElementSize();

  if (BufSize < ElemSize)
    return RustealFail(ERustealErrorCode::InternalError);

  Property->InitializeValue(OutBuf);
  Property->CopySingleValue(OutBuf, Src);
  return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetPropertyAtImpl(RustealUObjectHandle Obj,
                                           RustealFPropertyHandle Prop,
                                           uint32 Index, const uint8 *InBuf,
                                           uint32 BufSize) {
  RUSTEAL_CHECK_VALID(Obj);
  FProperty *Property = static_cast<FProperty *>(Prop.ptr);

  if (!Property)
    return RustealFail(ERustealErrorCode::PropertyNotFound);

  if (Index >= (uint32)Property->ArrayDim)
    return RustealFail(
        ERustealErrorCode::IndexOutOfRange,
        FString::Printf(TEXT("index %u past the %d elements of %s"), Index,
                        Property->ArrayDim, *RustealDescribe(Property)));

  void *Dest = Property->ContainerPtrToValuePtr<void>(Object, Index);
  uint32 ElemSize = Property->GetElementSize();

  if (BufSize < ElemSize)
    return RustealFail(ERustealErrorCode::InternalError);

  Property->CopySingleValue(Dest, InBuf);
  return ERustealErrorCode::Ok;
}

FRustealPropertyApi GPropertyApi = {
    &GetBoolImpl,
    &SetBoolImpl,
    &GetI32Impl,
    &SetI32Impl,
    &GetI64Impl,
    &SetI64Impl,
    &GetU8Impl,
    &SetU8Impl,
    &GetF32Impl,
    &SetF32Impl,
    &GetF64Impl,
    &SetF64Impl,
    &GetStringImpl,
    &SetStringImpl,
    &GetFNameImpl,
    &SetFNameImpl,
    &GetObjectImpl,
    &SetObjectImpl,
    &GetEnumImpl,
    &SetEnumImpl,
    &GetStructImpl,
    &SetStructImpl,
    &GetPropertyAtImpl,
    &SetPropertyAtImpl,
    &GetSoftObjectPathImpl,
    &SetSoftObjectPathImpl,
    &GetTextImpl,
    &SetTextImpl,
};
