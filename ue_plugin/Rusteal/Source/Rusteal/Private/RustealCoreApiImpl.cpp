#include "RustealApiTable.h"
#include "RustealErrors.h"
#include "UObject/UObjectGlobals.h"
#include "RustealStrings.h"
#include "RustealUtf8.h"

static bool IsValidImpl(RustealUObjectHandle Obj) {
  return ::IsValid(static_cast<UObject *>(Obj.ptr));
}

static ERustealErrorCode GetNameImpl(RustealUObjectHandle Obj, uint8 *Buf,
                                     uint32 BufLen, uint32 *OutLen) {
  UObject *Object = static_cast<UObject *>(Obj.ptr);

  if (!::IsValid(Object)) {
    return RustealFail(ERustealErrorCode::ObjectDestroyed);
  }

  const FString Name = Object->GetFName().ToString();
  const FTCHARToUTF8 Utf8(*Name);
  const uint32 Len = static_cast<uint32>(Utf8.Length());

  if (OutLen) {
    *OutLen = Len;
  }

  if (Buf && BufLen > 0) {
    const uint32 CopyLen = FMath::Min(Len, BufLen);
    FMemory::Memcpy(Buf, Utf8.Get(), CopyLen);
  }

  return ERustealErrorCode::Ok;
}

static RustealUClassHandle GetClassImpl(RustealUObjectHandle Obj) {
  UObject *Object = static_cast<UObject *>(Obj.ptr);

  if (!::IsValid(Object)) {
    return RustealUClassHandle{nullptr};
  }

  return RustealUClassHandle{Object->GetClass()};
}

static bool IsAImpl(RustealUObjectHandle Obj, RustealUClassHandle TargetClass) {
  UObject *Object = static_cast<UObject *>(Obj.ptr);

  if (!::IsValid(Object) || !TargetClass.ptr) {
    return false;
  }

  UClass *Target = static_cast<UClass *>(TargetClass.ptr);

  if (Target->HasAnyClassFlags(CLASS_Interface)) {
    return Object->GetClass()->ImplementsInterface(Target);
  }

  return Object->IsA(Target);
}

static RustealUObjectHandle GetOuterImpl(RustealUObjectHandle Obj) {
  UObject *Object = static_cast<UObject *>(Obj.ptr);

  if (!::IsValid(Object)) {
    return RustealUObjectHandle{nullptr};
  }

  return RustealUObjectHandle{Object->GetOuter()};
}

static RustealFNameHandle MakeFNameImpl(const uint8 *NameUtf8, uint32 NameLen) {
  const FString Str = RustealUtf8ToFString(NameUtf8, NameLen);

  const FName Name(*Str);

  RustealFNameHandle Result;

  Result.value =
      static_cast<uint64>(Name.GetComparisonIndex().ToUnstableInt()) |
      (static_cast<uint64>(Name.GetNumber()) << 32);

  return Result;
}

static ERustealErrorCode FNameToStringImpl(RustealFNameHandle Handle,
                                           uint8 *Buf, uint32 BufLen,
                                           uint32 *OutLen) {
  const FNameEntryId CompIdx = FNameEntryId::FromUnstableInt(
      static_cast<uint32>(Handle.value & 0xFFFFFFFF));

  const int32 Number = static_cast<int32>(Handle.value >> 32);
  const FName Name(CompIdx, CompIdx, Number);

  const FString Str = Name.ToString();
  const FTCHARToUTF8 Utf8(*Str);
  const uint32 Len = static_cast<uint32>(Utf8.Length());

  if (OutLen) {
    *OutLen = Len;
  }

  if (Buf && BufLen > 0) {
    const uint32 CopyLen = FMath::Min(Len, BufLen);
    FMemory::Memcpy(Buf, Utf8.Get(), CopyLen);
  }

  return ERustealErrorCode::Ok;
}

static RustealFWeakObjectHandle MakeWeakImpl(RustealUObjectHandle Obj) {
  UObject *Object = static_cast<UObject *>(Obj.ptr);

  if (!::IsValid(Object)) {
    return RustealFWeakObjectHandle{-1, 0};
  }

  FWeakObjectPtr Weak(Object);

  RustealFWeakObjectHandle Result;
  FMemory::Memcpy(&Result, &Weak, sizeof(Result));
  return Result;
}

static RustealUObjectHandle
ResolveWeakImpl(RustealFWeakObjectHandle WeakHandle) {
  FWeakObjectPtr Weak;
  FMemory::Memcpy(&Weak, &WeakHandle, sizeof(WeakHandle));
  UObject *Resolved = Weak.Get();
  return RustealUObjectHandle{Resolved};
}

static bool IsWeakValidImpl(RustealFWeakObjectHandle WeakHandle) {
  FWeakObjectPtr Weak;
  FMemory::Memcpy(&Weak, &WeakHandle, sizeof(WeakHandle));
  return Weak.IsValid();
}

static thread_local TMap<const void *, TArray<uint8>> GPendingUtf8;

void RustealWriteUtf8(const FString &Value, void *Buf, uint32 BufLen,
                      uint32 *OutLen) {
  const FTCHARToUTF8 Utf8(*Value);
  const uint32 Len = static_cast<uint32>(Utf8.Length());

  if (OutLen) {
    *OutLen = Len;
  }

  if (!Buf) {
    return;
  }

  FMemory::Memcpy(Buf, Utf8.Get(), FMath::Min(Len, BufLen));

  if (Len > BufLen) {
    TArray<uint8> &Pending = GPendingUtf8.FindOrAdd(Buf);
    Pending.SetNumUninitialized(Len);
    FMemory::Memcpy(Pending.GetData(), Utf8.Get(), Len);
  } else {
    GPendingUtf8.Remove(Buf);
  }
}

static uint32 TakePendingUtf8Impl(const uint8 *Key, uint8 *Buf, uint32 BufLen) {
  TArray<uint8> Pending;

  if (!GPendingUtf8.RemoveAndCopyValue(Key, Pending)) {
    return 0;
  }

  const uint32 Len = static_cast<uint32>(Pending.Num());
  FMemory::Memcpy(Buf, Pending.GetData(), FMath::Min(Len, BufLen));
  return Len;
}

static void *TextFromStringImpl(const uint8 *Utf8, uint32 Len) {
  return new FText(FText::FromString(RustealUtf8ToFString(Utf8, Len)));
}

static void *TextLocalizedImpl(const uint8 *Namespace, uint32 NamespaceLen,
                               const uint8 *Key, uint32 KeyLen,
                               const uint8 *Source, uint32 SourceLen) {
  return new FText(FText::AsLocalizable_Advanced(
      RustealUtf8ToFString(Namespace, NamespaceLen),
      RustealUtf8ToFString(Key, KeyLen),
      RustealUtf8ToFString(Source, SourceLen)));
}

static void *TextCloneImpl(const void *Text) {
  return new FText(*static_cast<const FText *>(Text));
}

static void TextDestroyImpl(void *Text) { delete static_cast<FText *>(Text); }

static void TextToStringImpl(const void *Text, uint8 *Buf, uint32 BufLen,
                             uint32 *OutLen) {
  RustealWriteUtf8(static_cast<const FText *>(Text)->ToString(), Buf, BufLen,
                   OutLen);
}

static bool TextIsEmptyImpl(const void *Text) {
  return static_cast<const FText *>(Text)->IsEmpty();
}

static bool TextEqualImpl(const void *A, const void *B) {
  return static_cast<const FText *>(A)->EqualTo(*static_cast<const FText *>(B));
}

static bool IsChildOfImpl(RustealUClassHandle ClassHandle,
                          RustealUClassHandle ParentHandle) {
  UObject *ClassObject = static_cast<UObject *>(ClassHandle.ptr);
  UObject *ParentObject = static_cast<UObject *>(ParentHandle.ptr);

  if (!::IsValid(ClassObject) || !::IsValid(ParentObject)) {
    return false;
  }

  UClass *Class = Cast<UClass>(ClassObject);
  UClass *Parent = Cast<UClass>(ParentObject);

  if (!Class || !Parent) {
    return false;
  }

  if (Parent->HasAnyClassFlags(CLASS_Interface)) {
    return Class->ImplementsInterface(Parent);
  }

  return Class->IsChildOf(Parent);
}

FRustealCoreApi GCoreApi = {
    &IsValidImpl,
    &GetNameImpl,
    &GetClassImpl,
    &IsAImpl,
    &GetOuterImpl,
    &MakeFNameImpl,
    &FNameToStringImpl,
    &MakeWeakImpl,
    &ResolveWeakImpl,
    &IsWeakValidImpl,
    &RustealTakeErrorMessage,
    &IsChildOfImpl,
    &TakePendingUtf8Impl,
    &TextFromStringImpl,
    &TextLocalizedImpl,
    &TextCloneImpl,
    &TextDestroyImpl,
    &TextToStringImpl,
    &TextIsEmptyImpl,
    &TextEqualImpl,
};
