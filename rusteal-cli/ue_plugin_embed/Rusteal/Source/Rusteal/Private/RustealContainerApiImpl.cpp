// RustealContainerApiImpl.cpp — FRustealContainerApi implementation.
// Handles TArray, TMap, TSet operations across FFI via type-erased buffers.
// The C++ side inspects inner FProperty to dispatch element read/write correctly.

#include "RustealApiTable.h"
#include "UObject/UnrealType.h"
#include "UObject/UObjectGlobals.h"

// ---------------------------------------------------------------------------
// Validity macro (same as PropertyApiImpl)
// ---------------------------------------------------------------------------

// Null-check only — UObject validity is the caller's responsibility.
// This also works for temp container bases (non-UObject allocations).
#define RUSTEAL_CHECK_VALID(ObjHandle)                                     \
    void* Object = (ObjHandle).ptr;                                     \
    if (!Object)                                                        \
    {                                                                   \
        return ERustealErrorCode::ObjectDestroyed;                         \
    }

#define RUSTEAL_CHECK_VALID_I32(ObjHandle)                                 \
    void* Object = (ObjHandle).ptr;                                     \
    if (!Object)                                                        \
    {                                                                   \
        return -1;                                                      \
    }

// ---------------------------------------------------------------------------
// Raw-copyable check: types where TArray memory can be memcpy'd directly
// ---------------------------------------------------------------------------

static bool IsRawCopyableElement(FProperty* InnerProp)
{
    return !CastField<FStrProperty>(InnerProp)
        && !CastField<FTextProperty>(InnerProp)
        && !CastField<FObjectPropertyBase>(InnerProp)
        && !CastField<FStructProperty>(InnerProp);
}

// ---------------------------------------------------------------------------
// Element read/write dispatch
// ---------------------------------------------------------------------------

static void ReadElement(FProperty* InnerProp, const void* ElemPtr,
                        uint8* OutBuf, uint32 BufSize, uint32* OutWritten)
{
    if (FStrProperty* StrProp = CastField<FStrProperty>(InnerProp))
    {
        // String -> UTF-8 with length prefix: [u32 len][utf8 bytes]
        const FString& Str = StrProp->GetPropertyValue(ElemPtr);
        FTCHARToUTF8 Utf8(*Str);
        uint32 Len = static_cast<uint32>(Utf8.Length());
        FMemory::Memcpy(OutBuf, &Len, sizeof(uint32));
        uint32 CopyLen = FMath::Min(Len, BufSize - static_cast<uint32>(sizeof(uint32)));
        FMemory::Memcpy(OutBuf + sizeof(uint32), Utf8.Get(), CopyLen);
        if (OutWritten) *OutWritten = sizeof(uint32) + CopyLen;
    }
    else if (FTextProperty* TextProp = CastField<FTextProperty>(InnerProp))
    {
        const FText& Text = TextProp->GetPropertyValue(ElemPtr);
        FString Str = Text.ToString();
        FTCHARToUTF8 Utf8(*Str);
        uint32 Len = static_cast<uint32>(Utf8.Length());
        FMemory::Memcpy(OutBuf, &Len, sizeof(uint32));
        uint32 CopyLen = FMath::Min(Len, BufSize - static_cast<uint32>(sizeof(uint32)));
        FMemory::Memcpy(OutBuf + sizeof(uint32), Utf8.Get(), CopyLen);
        if (OutWritten) *OutWritten = sizeof(uint32) + CopyLen;
    }
    else if (FObjectPropertyBase* ObjProp = CastField<FObjectPropertyBase>(InnerProp))
    {
        // Object -> UObject* pointer (8 bytes)
        UObject* Obj = ObjProp->GetObjectPropertyValue(ElemPtr);
        FMemory::Memcpy(OutBuf, &Obj, sizeof(void*));
        if (OutWritten) *OutWritten = sizeof(void*);
    }
    else if (FStructProperty* StructProp = CastField<FStructProperty>(InnerProp))
    {
        StructProp->Struct->CopyScriptStruct(OutBuf, ElemPtr);
        if (OutWritten) *OutWritten = StructProp->GetSize();
    }
    else
    {
        // Primitives, enums, FName — raw memcpy
        uint32 Size = InnerProp->GetSize();
        FMemory::Memcpy(OutBuf, ElemPtr, FMath::Min(Size, BufSize));
        if (OutWritten) *OutWritten = Size;
    }
}

static void WriteElement(FProperty* InnerProp, void* ElemPtr,
                         const uint8* InBuf, uint32 BufSize)
{
    if (FStrProperty* StrProp = CastField<FStrProperty>(InnerProp))
    {
        // UTF-8 with length prefix: [u32 len][utf8 bytes]
        uint32 Len = 0;
        FMemory::Memcpy(&Len, InBuf, sizeof(uint32));
        const char* Utf8Str = reinterpret_cast<const char*>(InBuf + sizeof(uint32));
        FString Value(Len, UTF8_TO_TCHAR(Utf8Str));
        StrProp->SetPropertyValue(ElemPtr, Value);
    }
    else if (FTextProperty* TextProp = CastField<FTextProperty>(InnerProp))
    {
        uint32 Len = 0;
        FMemory::Memcpy(&Len, InBuf, sizeof(uint32));
        const char* Utf8Str = reinterpret_cast<const char*>(InBuf + sizeof(uint32));
        FString Value(Len, UTF8_TO_TCHAR(Utf8Str));
        TextProp->SetPropertyValue(ElemPtr, FText::FromString(Value));
    }
    else if (FObjectPropertyBase* ObjProp = CastField<FObjectPropertyBase>(InnerProp))
    {
        UObject* Obj = nullptr;
        FMemory::Memcpy(&Obj, InBuf, sizeof(void*));
        ObjProp->SetObjectPropertyValue(ElemPtr, Obj);
    }
    else if (FStructProperty* StructProp = CastField<FStructProperty>(InnerProp))
    {
        StructProp->Struct->CopyScriptStruct(ElemPtr, InBuf);
    }
    else
    {
        // Primitives, enums, FName — raw memcpy
        uint32 Size = InnerProp->GetSize();
        FMemory::Memcpy(ElemPtr, InBuf, FMath::Min(Size, BufSize));
    }
}

// ---------------------------------------------------------------------------
// Helper to get inner property from container property types
// ---------------------------------------------------------------------------

static FProperty* GetArrayInnerProp(FProperty* Prop)
{
    FArrayProperty* ArrayProp = CastField<FArrayProperty>(Prop);
    return ArrayProp ? ArrayProp->Inner : nullptr;
}

// ---------------------------------------------------------------------------
// TArray implementation
// ---------------------------------------------------------------------------

static int32 ArrayLenImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop)
{
    RUSTEAL_CHECK_VALID_I32(Obj);
    FArrayProperty* ArrayProp = CastField<FArrayProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!ArrayProp) return -1;

    FScriptArrayHelper Helper(ArrayProp, ArrayProp->ContainerPtrToValuePtr<void>(Object));
    return Helper.Num();
}

static ERustealErrorCode ArrayGetImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop,
                                    int32 Index, uint8* OutBuf, uint32 BufSize, uint32* OutWritten)
{
    RUSTEAL_CHECK_VALID(Obj);
    FArrayProperty* ArrayProp = CastField<FArrayProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!ArrayProp) return ERustealErrorCode::TypeMismatch;

    FScriptArrayHelper Helper(ArrayProp, ArrayProp->ContainerPtrToValuePtr<void>(Object));
    if (Index < 0 || Index >= Helper.Num())
    {
        return ERustealErrorCode::IndexOutOfRange;
    }

    const void* ElemPtr = Helper.GetRawPtr(Index);
    ReadElement(ArrayProp->Inner, ElemPtr, OutBuf, BufSize, OutWritten);
    return ERustealErrorCode::Ok;
}

static ERustealErrorCode ArraySetImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop,
                                    int32 Index, const uint8* InBuf, uint32 BufSize)
{
    RUSTEAL_CHECK_VALID(Obj);
    FArrayProperty* ArrayProp = CastField<FArrayProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!ArrayProp) return ERustealErrorCode::TypeMismatch;

    FScriptArrayHelper Helper(ArrayProp, ArrayProp->ContainerPtrToValuePtr<void>(Object));
    if (Index < 0 || Index >= Helper.Num())
    {
        return ERustealErrorCode::IndexOutOfRange;
    }

    void* ElemPtr = Helper.GetRawPtr(Index);
    WriteElement(ArrayProp->Inner, ElemPtr, InBuf, BufSize);
    return ERustealErrorCode::Ok;
}

static ERustealErrorCode ArrayAddImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop,
                                    const uint8* InBuf, uint32 BufSize)
{
    RUSTEAL_CHECK_VALID(Obj);
    FArrayProperty* ArrayProp = CastField<FArrayProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!ArrayProp) return ERustealErrorCode::TypeMismatch;

    FScriptArrayHelper Helper(ArrayProp, ArrayProp->ContainerPtrToValuePtr<void>(Object));
    int32 NewIndex = Helper.AddValue();
    void* ElemPtr = Helper.GetRawPtr(NewIndex);
    WriteElement(ArrayProp->Inner, ElemPtr, InBuf, BufSize);
    return ERustealErrorCode::Ok;
}

static ERustealErrorCode ArrayRemoveImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop, int32 Index)
{
    RUSTEAL_CHECK_VALID(Obj);
    FArrayProperty* ArrayProp = CastField<FArrayProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!ArrayProp) return ERustealErrorCode::TypeMismatch;

    FScriptArrayHelper Helper(ArrayProp, ArrayProp->ContainerPtrToValuePtr<void>(Object));
    if (Index < 0 || Index >= Helper.Num())
    {
        return ERustealErrorCode::IndexOutOfRange;
    }

    Helper.RemoveValues(Index, 1);
    return ERustealErrorCode::Ok;
}

static ERustealErrorCode ArrayClearImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop)
{
    RUSTEAL_CHECK_VALID(Obj);
    FArrayProperty* ArrayProp = CastField<FArrayProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!ArrayProp) return ERustealErrorCode::TypeMismatch;

    FScriptArrayHelper Helper(ArrayProp, ArrayProp->ContainerPtrToValuePtr<void>(Object));
    Helper.EmptyValues();
    return ERustealErrorCode::Ok;
}

static uint32 ArrayElementSizeImpl(RustealFPropertyHandle Prop)
{
    FArrayProperty* ArrayProp = CastField<FArrayProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!ArrayProp || !ArrayProp->Inner) return 0;
    return ArrayProp->Inner->GetSize();
}

// ---------------------------------------------------------------------------
// TMap implementation
// ---------------------------------------------------------------------------

static int32 MapLenImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop)
{
    RUSTEAL_CHECK_VALID_I32(Obj);
    FMapProperty* MapProp = CastField<FMapProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!MapProp) return -1;

    FScriptMapHelper Helper(MapProp, MapProp->ContainerPtrToValuePtr<void>(Object));
    return Helper.Num();
}

static ERustealErrorCode MapFindImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop,
                                   const uint8* KeyBuf, uint32 KeySize,
                                   uint8* OutValBuf, uint32 ValSize, uint32* OutWritten)
{
    RUSTEAL_CHECK_VALID(Obj);
    FMapProperty* MapProp = CastField<FMapProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!MapProp) return ERustealErrorCode::TypeMismatch;

    FScriptMapHelper Helper(MapProp, MapProp->ContainerPtrToValuePtr<void>(Object));

    // Build a temporary key to search with
    FProperty* KeyProp = MapProp->KeyProp;
    uint8* TempKey = static_cast<uint8*>(FMemory::Malloc(KeyProp->GetSize(), KeyProp->GetMinAlignment()));
    KeyProp->InitializeValue(TempKey);
    WriteElement(KeyProp, TempKey, KeyBuf, KeySize);

    // Search for the key
    const uint8* FoundValuePtr = Helper.FindValueFromHash(TempKey);

    if (!FoundValuePtr)
    {
        KeyProp->DestroyValue(TempKey);
        FMemory::Free(TempKey);
        return ERustealErrorCode::PropertyNotFound;
    }

    ReadElement(MapProp->ValueProp, FoundValuePtr, OutValBuf, ValSize, OutWritten);

    KeyProp->DestroyValue(TempKey);
    FMemory::Free(TempKey);
    return ERustealErrorCode::Ok;
}

static ERustealErrorCode MapAddImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop,
                                  const uint8* KeyBuf, uint32 KeySize,
                                  const uint8* ValBuf, uint32 ValSize)
{
    RUSTEAL_CHECK_VALID(Obj);
    FMapProperty* MapProp = CastField<FMapProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!MapProp) return ERustealErrorCode::TypeMismatch;

    FScriptMapHelper Helper(MapProp, MapProp->ContainerPtrToValuePtr<void>(Object));

    FProperty* KeyProp = MapProp->KeyProp;
    FProperty* ValueProp = MapProp->ValueProp;

    // Build temporary key
    uint8* TempKey = static_cast<uint8*>(FMemory::Malloc(KeyProp->GetSize(), KeyProp->GetMinAlignment()));
    KeyProp->InitializeValue(TempKey);
    WriteElement(KeyProp, TempKey, KeyBuf, KeySize);

    // Build temporary value
    uint8* TempVal = static_cast<uint8*>(FMemory::Malloc(ValueProp->GetSize(), ValueProp->GetMinAlignment()));
    ValueProp->InitializeValue(TempVal);
    WriteElement(ValueProp, TempVal, ValBuf, ValSize);

    // Add to map (replaces existing key if present)
    Helper.AddPair(TempKey, TempVal);

    KeyProp->DestroyValue(TempKey);
    FMemory::Free(TempKey);
    ValueProp->DestroyValue(TempVal);
    FMemory::Free(TempVal);
    return ERustealErrorCode::Ok;
}

static ERustealErrorCode MapRemoveImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop,
                                     const uint8* KeyBuf, uint32 KeySize)
{
    RUSTEAL_CHECK_VALID(Obj);
    FMapProperty* MapProp = CastField<FMapProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!MapProp) return ERustealErrorCode::TypeMismatch;

    FScriptMapHelper Helper(MapProp, MapProp->ContainerPtrToValuePtr<void>(Object));

    FProperty* KeyProp = MapProp->KeyProp;
    uint8* TempKey = static_cast<uint8*>(FMemory::Malloc(KeyProp->GetSize(), KeyProp->GetMinAlignment()));
    KeyProp->InitializeValue(TempKey);
    WriteElement(KeyProp, TempKey, KeyBuf, KeySize);

    bool bRemoved = Helper.RemovePair(TempKey);

    KeyProp->DestroyValue(TempKey);
    FMemory::Free(TempKey);

    return bRemoved ? ERustealErrorCode::Ok : ERustealErrorCode::PropertyNotFound;
}

static ERustealErrorCode MapClearImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop)
{
    RUSTEAL_CHECK_VALID(Obj);
    FMapProperty* MapProp = CastField<FMapProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!MapProp) return ERustealErrorCode::TypeMismatch;

    FScriptMapHelper Helper(MapProp, MapProp->ContainerPtrToValuePtr<void>(Object));
    Helper.EmptyValues();
    return ERustealErrorCode::Ok;
}

static ERustealErrorCode MapGetPairImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop,
                                      int32 LogicalIndex,
                                      uint8* OutKeyBuf, uint32 KeyBufSize, uint32* OutKeyWritten,
                                      uint8* OutValBuf, uint32 ValBufSize, uint32* OutValWritten)
{
    RUSTEAL_CHECK_VALID(Obj);
    FMapProperty* MapProp = CastField<FMapProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!MapProp) return ERustealErrorCode::TypeMismatch;

    FScriptMapHelper Helper(MapProp, MapProp->ContainerPtrToValuePtr<void>(Object));

    if (LogicalIndex < 0 || LogicalIndex >= Helper.Num())
    {
        return ERustealErrorCode::IndexOutOfRange;
    }

    // Skip to the Nth valid entry (sparse map layout)
    int32 Found = 0;
    int32 MaxIndex = Helper.GetMaxIndex();
    for (int32 i = 0; i < MaxIndex; ++i)
    {
        if (Helper.IsValidIndex(i))
        {
            if (Found == LogicalIndex)
            {
                ReadElement(MapProp->KeyProp, Helper.GetKeyPtr(i), OutKeyBuf, KeyBufSize, OutKeyWritten);
                ReadElement(MapProp->ValueProp, Helper.GetValuePtr(i), OutValBuf, ValBufSize, OutValWritten);
                return ERustealErrorCode::Ok;
            }
            ++Found;
        }
    }

    return ERustealErrorCode::IndexOutOfRange;
}

// ---------------------------------------------------------------------------
// TSet implementation
// ---------------------------------------------------------------------------

static int32 SetLenImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop)
{
    RUSTEAL_CHECK_VALID_I32(Obj);
    FSetProperty* SetProp = CastField<FSetProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!SetProp) return -1;

    FScriptSetHelper Helper(SetProp, SetProp->ContainerPtrToValuePtr<void>(Object));
    return Helper.Num();
}

static bool SetContainsImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop,
                             const uint8* ElemBuf, uint32 ElemSize)
{
    UObject* Object = static_cast<UObject*>(Obj.ptr);
    if (!::IsValid(Object)) return false;

    FSetProperty* SetProp = CastField<FSetProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!SetProp) return false;

    FScriptSetHelper Helper(SetProp, SetProp->ContainerPtrToValuePtr<void>(Object));

    FProperty* ElementProp = SetProp->ElementProp;
    uint8* TempElem = static_cast<uint8*>(FMemory::Malloc(ElementProp->GetSize(), ElementProp->GetMinAlignment()));
    ElementProp->InitializeValue(TempElem);
    WriteElement(ElementProp, TempElem, ElemBuf, ElemSize);

    int32 FoundIndex = Helper.FindElementIndexFromHash(TempElem);

    ElementProp->DestroyValue(TempElem);
    FMemory::Free(TempElem);

    return FoundIndex != INDEX_NONE;
}

static ERustealErrorCode SetAddImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop,
                                  const uint8* ElemBuf, uint32 ElemSize)
{
    RUSTEAL_CHECK_VALID(Obj);
    FSetProperty* SetProp = CastField<FSetProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!SetProp) return ERustealErrorCode::TypeMismatch;

    FScriptSetHelper Helper(SetProp, SetProp->ContainerPtrToValuePtr<void>(Object));

    FProperty* ElementProp = SetProp->ElementProp;
    uint8* TempElem = static_cast<uint8*>(FMemory::Malloc(ElementProp->GetSize(), ElementProp->GetMinAlignment()));
    ElementProp->InitializeValue(TempElem);
    WriteElement(ElementProp, TempElem, ElemBuf, ElemSize);

    Helper.AddElement(TempElem);

    ElementProp->DestroyValue(TempElem);
    FMemory::Free(TempElem);
    return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetRemoveImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop,
                                     const uint8* ElemBuf, uint32 ElemSize)
{
    RUSTEAL_CHECK_VALID(Obj);
    FSetProperty* SetProp = CastField<FSetProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!SetProp) return ERustealErrorCode::TypeMismatch;

    FScriptSetHelper Helper(SetProp, SetProp->ContainerPtrToValuePtr<void>(Object));

    FProperty* ElementProp = SetProp->ElementProp;
    uint8* TempElem = static_cast<uint8*>(FMemory::Malloc(ElementProp->GetSize(), ElementProp->GetMinAlignment()));
    ElementProp->InitializeValue(TempElem);
    WriteElement(ElementProp, TempElem, ElemBuf, ElemSize);

    int32 FoundIndex = Helper.FindElementIndexFromHash(TempElem);
    bool bRemoved = false;
    if (FoundIndex != INDEX_NONE)
    {
        Helper.RemoveAt(FoundIndex);
        bRemoved = true;
    }

    ElementProp->DestroyValue(TempElem);
    FMemory::Free(TempElem);
    return bRemoved ? ERustealErrorCode::Ok : ERustealErrorCode::PropertyNotFound;
}

static ERustealErrorCode SetClearImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop)
{
    RUSTEAL_CHECK_VALID(Obj);
    FSetProperty* SetProp = CastField<FSetProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!SetProp) return ERustealErrorCode::TypeMismatch;

    FScriptSetHelper Helper(SetProp, SetProp->ContainerPtrToValuePtr<void>(Object));
    Helper.EmptyElements();
    return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetGetElementImpl(RustealUObjectHandle Obj, RustealFPropertyHandle Prop,
                                         int32 LogicalIndex,
                                         uint8* OutBuf, uint32 BufSize, uint32* OutWritten)
{
    RUSTEAL_CHECK_VALID(Obj);
    FSetProperty* SetProp = CastField<FSetProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!SetProp) return ERustealErrorCode::TypeMismatch;

    FScriptSetHelper Helper(SetProp, SetProp->ContainerPtrToValuePtr<void>(Object));

    if (LogicalIndex < 0 || LogicalIndex >= Helper.Num())
    {
        return ERustealErrorCode::IndexOutOfRange;
    }

    // Skip to the Nth valid entry (sparse set layout)
    int32 Found = 0;
    int32 MaxIndex = Helper.GetMaxIndex();
    for (int32 i = 0; i < MaxIndex; ++i)
    {
        if (Helper.IsValidIndex(i))
        {
            if (Found == LogicalIndex)
            {
                ReadElement(SetProp->ElementProp, Helper.GetElementPtr(i), OutBuf, BufSize, OutWritten);
                return ERustealErrorCode::Ok;
            }
            ++Found;
        }
    }

    return ERustealErrorCode::IndexOutOfRange;
}

// ---------------------------------------------------------------------------
// Bulk copy/set — single FFI call for entire container
// ---------------------------------------------------------------------------
// Format: [u32 written_1][data_1][u32 written_2][data_2]...
// For maps: [u32 key_written][key_data][u32 val_written][val_data] per pair.

static ERustealErrorCode ArrayCopyAllImpl(
    RustealUObjectHandle Obj, RustealFPropertyHandle Prop,
    uint8* OutBuf, uint32 BufSize, uint32* OutTotalWritten, int32* OutCount)
{
    RUSTEAL_CHECK_VALID(Obj);
    FArrayProperty* ArrayProp = CastField<FArrayProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!ArrayProp) return ERustealErrorCode::TypeMismatch;

    FScriptArrayHelper Helper(ArrayProp, ArrayProp->ContainerPtrToValuePtr<void>(Object));
    int32 Count = Helper.Num();
    if (OutCount) *OutCount = Count;

    FProperty* Inner = ArrayProp->Inner;

    // Fast path: raw memcpy for fixed-size primitive types
    if (IsRawCopyableElement(Inner))
    {
        uint32 ElemSize = Inner->GetSize();
        uint32 TotalSize = Count * ElemSize;
        if (TotalSize > BufSize)
        {
            if (OutTotalWritten) *OutTotalWritten = TotalSize;
            return ERustealErrorCode::BufferTooSmall;
        }
        if (Count > 0)
        {
            FMemory::Memcpy(OutBuf, Helper.GetRawPtr(0), TotalSize);
        }
        if (OutTotalWritten) *OutTotalWritten = TotalSize;
        if (OutCount) *OutCount = -Count;  // negative = raw format
        return ERustealErrorCode::Ok;
    }

    // Framed path: [u32 written][data] per element
    uint32 Offset = 0;

    for (int32 i = 0; i < Count; i++)
    {
        // Check if we have room for at least the length prefix
        if (Offset + sizeof(uint32) > BufSize)
        {
            // Estimate total size needed: extrapolate from what we've written so far
            if (OutTotalWritten && i > 0)
            {
                uint32 AvgPerElem = Offset / i;
                *OutTotalWritten = AvgPerElem * Count + sizeof(uint32) * Count;
            }
            return ERustealErrorCode::BufferTooSmall;
        }

        uint32 ElemWritten = 0;
        uint32 Remaining = BufSize - Offset - sizeof(uint32);
        ReadElement(Inner, Helper.GetRawPtr(i),
                    OutBuf + Offset + sizeof(uint32), Remaining, &ElemWritten);

        if (Offset + sizeof(uint32) + ElemWritten > BufSize)
        {
            // Element was partially written or didn't fit
            if (OutTotalWritten && i > 0)
            {
                uint32 AvgPerElem = (Offset + sizeof(uint32) + ElemWritten) / (i + 1);
                *OutTotalWritten = AvgPerElem * Count;
            }
            else if (OutTotalWritten)
            {
                *OutTotalWritten = (sizeof(uint32) + ElemWritten) * Count;
            }
            return ERustealErrorCode::BufferTooSmall;
        }

        FMemory::Memcpy(OutBuf + Offset, &ElemWritten, sizeof(uint32));
        Offset += sizeof(uint32) + ElemWritten;
    }

    if (OutTotalWritten) *OutTotalWritten = Offset;
    return ERustealErrorCode::Ok;
}

static ERustealErrorCode ArraySetAllImpl(
    RustealUObjectHandle Obj, RustealFPropertyHandle Prop,
    const uint8* InBuf, uint32 BufSize, int32 Count)
{
    RUSTEAL_CHECK_VALID(Obj);
    FArrayProperty* ArrayProp = CastField<FArrayProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!ArrayProp) return ERustealErrorCode::TypeMismatch;

    FScriptArrayHelper Helper(ArrayProp, ArrayProp->ContainerPtrToValuePtr<void>(Object));
    FProperty* Inner = ArrayProp->Inner;

    // Fast path: raw memcpy for fixed-size primitive types (negative count = raw format)
    if (Count < 0)
    {
        int32 ActualCount = -Count;
        uint32 ElemSize = Inner->GetSize();
        uint32 TotalSize = ActualCount * ElemSize;
        if (TotalSize > BufSize) return ERustealErrorCode::BufferTooSmall;
        Helper.EmptyValues();
        Helper.Resize(ActualCount);
        if (ActualCount > 0)
        {
            FMemory::Memcpy(Helper.GetRawPtr(0), InBuf, TotalSize);
        }
        return ERustealErrorCode::Ok;
    }

    // Framed path: [u32 written][data] per element
    Helper.EmptyValues();
    Helper.Resize(Count);

    uint32 Offset = 0;
    for (int32 i = 0; i < Count; i++)
    {
        if (Offset + sizeof(uint32) > BufSize)
        {
            return ERustealErrorCode::BufferTooSmall;
        }

        uint32 ElemSize = 0;
        FMemory::Memcpy(&ElemSize, InBuf + Offset, sizeof(uint32));
        Offset += sizeof(uint32);

        if (Offset + ElemSize > BufSize)
        {
            return ERustealErrorCode::BufferTooSmall;
        }

        WriteElement(Inner, Helper.GetRawPtr(i), InBuf + Offset, ElemSize);
        Offset += ElemSize;
    }

    return ERustealErrorCode::Ok;
}

static ERustealErrorCode MapCopyAllImpl(
    RustealUObjectHandle Obj, RustealFPropertyHandle Prop,
    uint8* OutBuf, uint32 BufSize, uint32* OutTotalWritten, int32* OutCount)
{
    RUSTEAL_CHECK_VALID(Obj);
    FMapProperty* MapProp = CastField<FMapProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!MapProp) return ERustealErrorCode::TypeMismatch;

    FScriptMapHelper Helper(MapProp, MapProp->ContainerPtrToValuePtr<void>(Object));
    int32 Count = Helper.Num();
    if (OutCount) *OutCount = Count;

    FProperty* KeyProp = MapProp->KeyProp;
    FProperty* ValueProp = MapProp->ValueProp;
    uint32 Offset = 0;
    int32 MaxIndex = Helper.GetMaxIndex();

    for (int32 i = 0; i < MaxIndex; ++i)
    {
        if (!Helper.IsValidIndex(i)) continue;

        // Key: [u32 written][data]
        if (Offset + sizeof(uint32) > BufSize)
        {
            if (OutTotalWritten) *OutTotalWritten = BufSize * 2; // rough estimate
            return ERustealErrorCode::BufferTooSmall;
        }

        uint32 KeyWritten = 0;
        uint32 Remaining = BufSize - Offset - sizeof(uint32);
        ReadElement(KeyProp, Helper.GetKeyPtr(i),
                    OutBuf + Offset + sizeof(uint32), Remaining, &KeyWritten);

        if (Offset + sizeof(uint32) + KeyWritten > BufSize)
        {
            if (OutTotalWritten) *OutTotalWritten = BufSize * 2;
            return ERustealErrorCode::BufferTooSmall;
        }

        FMemory::Memcpy(OutBuf + Offset, &KeyWritten, sizeof(uint32));
        Offset += sizeof(uint32) + KeyWritten;

        // Value: [u32 written][data]
        if (Offset + sizeof(uint32) > BufSize)
        {
            if (OutTotalWritten) *OutTotalWritten = BufSize * 2;
            return ERustealErrorCode::BufferTooSmall;
        }

        uint32 ValWritten = 0;
        Remaining = BufSize - Offset - sizeof(uint32);
        ReadElement(ValueProp, Helper.GetValuePtr(i),
                    OutBuf + Offset + sizeof(uint32), Remaining, &ValWritten);

        if (Offset + sizeof(uint32) + ValWritten > BufSize)
        {
            if (OutTotalWritten) *OutTotalWritten = BufSize * 2;
            return ERustealErrorCode::BufferTooSmall;
        }

        FMemory::Memcpy(OutBuf + Offset, &ValWritten, sizeof(uint32));
        Offset += sizeof(uint32) + ValWritten;
    }

    if (OutTotalWritten) *OutTotalWritten = Offset;
    return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetCopyAllImpl(
    RustealUObjectHandle Obj, RustealFPropertyHandle Prop,
    uint8* OutBuf, uint32 BufSize, uint32* OutTotalWritten, int32* OutCount)
{
    RUSTEAL_CHECK_VALID(Obj);
    FSetProperty* SetProp = CastField<FSetProperty>(static_cast<FProperty*>(Prop.ptr));
    if (!SetProp) return ERustealErrorCode::TypeMismatch;

    FScriptSetHelper Helper(SetProp, SetProp->ContainerPtrToValuePtr<void>(Object));
    int32 Count = Helper.Num();
    if (OutCount) *OutCount = Count;

    FProperty* ElementProp = SetProp->ElementProp;
    uint32 Offset = 0;
    int32 MaxIndex = Helper.GetMaxIndex();

    for (int32 i = 0; i < MaxIndex; ++i)
    {
        if (!Helper.IsValidIndex(i)) continue;

        if (Offset + sizeof(uint32) > BufSize)
        {
            if (OutTotalWritten) *OutTotalWritten = BufSize * 2;
            return ERustealErrorCode::BufferTooSmall;
        }

        uint32 ElemWritten = 0;
        uint32 Remaining = BufSize - Offset - sizeof(uint32);
        ReadElement(ElementProp, Helper.GetElementPtr(i),
                    OutBuf + Offset + sizeof(uint32), Remaining, &ElemWritten);

        if (Offset + sizeof(uint32) + ElemWritten > BufSize)
        {
            if (OutTotalWritten) *OutTotalWritten = BufSize * 2;
            return ERustealErrorCode::BufferTooSmall;
        }

        FMemory::Memcpy(OutBuf + Offset, &ElemWritten, sizeof(uint32));
        Offset += sizeof(uint32) + ElemWritten;
    }

    if (OutTotalWritten) *OutTotalWritten = Offset;
    return ERustealErrorCode::Ok;
}

// ---------------------------------------------------------------------------
// Temp container allocation (for function params)
// ---------------------------------------------------------------------------

static void* AllocTempImpl(RustealFPropertyHandle Prop)
{
    FProperty* PropPtr = static_cast<FProperty*>(Prop.ptr);
    if (!PropPtr) return nullptr;

    uint32 Offset = PropPtr->GetOffset_ForUFunction();
    uint32 Size = PropPtr->GetSize();

    // Allocate: [offset padding][container memory]
    // so that ContainerPtrToValuePtr(Base) = Base + Offset = container
    uint8* Base = static_cast<uint8*>(FMemory::Malloc(Offset + Size));
    FMemory::Memzero(Base, Offset + Size);
    PropPtr->InitializeValue(Base + Offset);
    return Base;
}

static void FreeTempImpl(RustealFPropertyHandle Prop, void* Base)
{
    if (!Base) return;
    FProperty* PropPtr = static_cast<FProperty*>(Prop.ptr);
    if (!PropPtr)
    {
        FMemory::Free(Base);
        return;
    }

    uint32 Offset = PropPtr->GetOffset_ForUFunction();
    PropPtr->DestroyValue(static_cast<uint8*>(Base) + Offset);
    FMemory::Free(Base);
}

// ---------------------------------------------------------------------------
// Static instance
// ---------------------------------------------------------------------------

FRustealContainerApi GContainerApi = {
    // TArray
    &ArrayLenImpl,
    &ArrayGetImpl,
    &ArraySetImpl,
    &ArrayAddImpl,
    &ArrayRemoveImpl,
    &ArrayClearImpl,
    &ArrayElementSizeImpl,
    // TMap
    &MapLenImpl,
    &MapFindImpl,
    &MapAddImpl,
    &MapRemoveImpl,
    &MapClearImpl,
    &MapGetPairImpl,
    // TSet
    &SetLenImpl,
    &SetContainsImpl,
    &SetAddImpl,
    &SetRemoveImpl,
    &SetClearImpl,
    &SetGetElementImpl,
    // Temp allocation
    &AllocTempImpl,
    &FreeTempImpl,
    // Bulk copy/set
    &ArrayCopyAllImpl,
    &ArraySetAllImpl,
    &MapCopyAllImpl,
    &SetCopyAllImpl,
};
