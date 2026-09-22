// RustealCoreApiImpl.cpp — FRustealCoreApi implementation.

#include "RustealApiTable.h"
#include "UObject/UObjectGlobals.h"

static bool IsValidImpl(RustealUObjectHandle Obj)
{
    return ::IsValid(static_cast<UObject*>(Obj.ptr));
}

static ERustealErrorCode GetNameImpl(RustealUObjectHandle Obj, uint8* Buf, uint32 BufLen, uint32* OutLen)
{
    UObject* Object = static_cast<UObject*>(Obj.ptr);
    if (!::IsValid(Object))
    {
        return ERustealErrorCode::ObjectDestroyed;
    }

    const FString Name = Object->GetFName().ToString();
    const FTCHARToUTF8 Utf8(*Name);
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

static RustealUClassHandle GetClassImpl(RustealUObjectHandle Obj)
{
    UObject* Object = static_cast<UObject*>(Obj.ptr);
    if (!::IsValid(Object))
    {
        return RustealUClassHandle{ nullptr };
    }
    return RustealUClassHandle{ Object->GetClass() };
}

static bool IsAImpl(RustealUObjectHandle Obj, RustealUClassHandle TargetClass)
{
    UObject* Object = static_cast<UObject*>(Obj.ptr);
    if (!::IsValid(Object) || !TargetClass.ptr)
    {
        return false;
    }
    return Object->IsA(static_cast<UClass*>(TargetClass.ptr));
}

static RustealUObjectHandle GetOuterImpl(RustealUObjectHandle Obj)
{
    UObject* Object = static_cast<UObject*>(Obj.ptr);
    if (!::IsValid(Object))
    {
        return RustealUObjectHandle{ nullptr };
    }
    return RustealUObjectHandle{ Object->GetOuter() };
}

// ---------------------------------------------------------------------------
// FName construction / conversion
// ---------------------------------------------------------------------------

static RustealFNameHandle MakeFNameImpl(const uint8* NameUtf8, uint32 NameLen)
{
    const FString Str(NameLen, UTF8_TO_TCHAR(reinterpret_cast<const char*>(NameUtf8)));
    const FName Name(*Str);
    // Pack FName into a uint64: ComparisonIndex in low 32, Number in high 32.
    RustealFNameHandle Result;
    Result.value = static_cast<uint64>(Name.GetComparisonIndex().ToUnstableInt())
                 | (static_cast<uint64>(Name.GetNumber()) << 32);
    return Result;
}

static ERustealErrorCode FNameToStringImpl(RustealFNameHandle Handle, uint8* Buf, uint32 BufLen, uint32* OutLen)
{
    // Reconstruct FName from packed uint64.
    const FNameEntryId CompIdx = FNameEntryId::FromUnstableInt(static_cast<uint32>(Handle.value & 0xFFFFFFFF));
    const int32 Number = static_cast<int32>(Handle.value >> 32);
    const FName Name(CompIdx, CompIdx, Number);

    const FString Str = Name.ToString();
    const FTCHARToUTF8 Utf8(*Str);
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

// ---------------------------------------------------------------------------
// Weak object pointers
// ---------------------------------------------------------------------------

static RustealFWeakObjectHandle MakeWeakImpl(RustealUObjectHandle Obj)
{
    UObject* Object = static_cast<UObject*>(Obj.ptr);
    if (!::IsValid(Object))
    {
        return RustealFWeakObjectHandle{ -1, 0 };
    }
    FWeakObjectPtr Weak(Object);
    // FWeakObjectPtr stores ObjectIndex and ObjectSerialNumber internally.
    // Access via the Get() approach — we store the index/serial from the internal state.
    RustealFWeakObjectHandle Result;
    FMemory::Memcpy(&Result, &Weak, sizeof(Result));
    return Result;
}

static RustealUObjectHandle ResolveWeakImpl(RustealFWeakObjectHandle WeakHandle)
{
    FWeakObjectPtr Weak;
    FMemory::Memcpy(&Weak, &WeakHandle, sizeof(WeakHandle));
    UObject* Resolved = Weak.Get();
    return RustealUObjectHandle{ Resolved };
}

static bool IsWeakValidImpl(RustealFWeakObjectHandle WeakHandle)
{
    FWeakObjectPtr Weak;
    FMemory::Memcpy(&Weak, &WeakHandle, sizeof(WeakHandle));
    return Weak.IsValid();
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
};
