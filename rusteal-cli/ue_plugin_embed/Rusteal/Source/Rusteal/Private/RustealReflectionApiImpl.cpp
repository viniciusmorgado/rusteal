// RustealReflectionApiImpl.cpp — FRustealReflectionApi implementation.

#include "RustealApiTable.h"
#include "UObject/UObjectGlobals.h"
#include "UObject/UnrealType.h"

// Helper: convert UTF-8 byte slice to FName.
static FName Utf8ToFName(const uint8* Name, uint32 NameLen)
{
    const FString Str(NameLen, UTF8_TO_TCHAR(reinterpret_cast<const char*>(Name)));
    return FName(*Str);
}

// Helper: convert UTF-8 byte slice to FString.
static FString Utf8ToFString(const uint8* Name, uint32 NameLen)
{
    return FString(NameLen, UTF8_TO_TCHAR(reinterpret_cast<const char*>(Name)));
}

// ---------------------------------------------------------------------------
// Implementations
// ---------------------------------------------------------------------------

static RustealUClassHandle FindClassImpl(const uint8* Name, uint32 NameLen)
{
    const FString ClassName = Utf8ToFString(Name, NameLen);
    UClass* Found = FindFirstObject<UClass>(*ClassName, EFindFirstObjectOptions::NativeFirst);
    return RustealUClassHandle{ Found };
}

static RustealFPropertyHandle FindPropertyImpl(RustealUClassHandle Cls, const uint8* Name, uint32 NameLen)
{
    UClass* Class = static_cast<UClass*>(Cls.ptr);
    if (!Class)
    {
        return RustealFPropertyHandle{ nullptr };
    }
    const FName PropName = Utf8ToFName(Name, NameLen);
    FProperty* Prop = Class->FindPropertyByName(PropName);
    return RustealFPropertyHandle{ Prop };
}

static RustealUClassHandle GetStaticClassImpl(const uint8* Name, uint32 NameLen)
{
    // Same as FindClass — generated code passes the class short name.
    return FindClassImpl(Name, NameLen);
}

static uint32 GetPropertySizeImpl(RustealFPropertyHandle Prop)
{
    FProperty* Property = static_cast<FProperty*>(Prop.ptr);
    if (!Property)
    {
        return 0;
    }
    return static_cast<uint32>(Property->GetSize());
}

static RustealUStructHandle FindStructImpl(const uint8* Name, uint32 NameLen)
{
    const FString StructName = Utf8ToFString(Name, NameLen);
    UScriptStruct* Found = FindFirstObject<UScriptStruct>(*StructName, EFindFirstObjectOptions::NativeFirst);
    return RustealUStructHandle{ Found };
}

static RustealFPropertyHandle FindStructPropertyImpl(RustealUStructHandle UStruct, const uint8* Name, uint32 NameLen)
{
    UScriptStruct* Struct = static_cast<UScriptStruct*>(UStruct.ptr);
    if (!Struct)
    {
        return RustealFPropertyHandle{ nullptr };
    }
    const FName PropName = Utf8ToFName(Name, NameLen);
    FProperty* Prop = Struct->FindPropertyByName(PropName);
    return RustealFPropertyHandle{ Prop };
}

// ---------------------------------------------------------------------------
// Dynamic call implementations (Phase 6)
// ---------------------------------------------------------------------------

static RustealUFunctionHandle FindFunctionImpl(RustealUObjectHandle Obj, const uint8* Name, uint32 NameLen)
{
    UObject* Object = static_cast<UObject*>(Obj.ptr);
    if (!::IsValid(Object)) return RustealUFunctionHandle{ nullptr };
    const FName FuncName = Utf8ToFName(Name, NameLen);
    UFunction* Func = Object->FindFunction(FuncName);
    return RustealUFunctionHandle{ Func };
}

static uint8* AllocParamsImpl(RustealUFunctionHandle Func)
{
    UFunction* Function = static_cast<UFunction*>(Func.ptr);
    if (!Function || Function->ParmsSize == 0) return nullptr;
    uint8* Params = static_cast<uint8*>(FMemory::Malloc(Function->ParmsSize));
    FMemory::Memzero(Params, Function->ParmsSize);
    // Initialize properties to defaults
    for (TFieldIterator<FProperty> It(Function); It && It->HasAnyPropertyFlags(CPF_Parm); ++It)
    {
        It->InitializeValue_InContainer(Params);
    }
    return Params;
}

static void FreeParamsImpl(RustealUFunctionHandle Func, uint8* Params)
{
    if (!Params) return;
    UFunction* Function = static_cast<UFunction*>(Func.ptr);
    if (Function)
    {
        for (TFieldIterator<FProperty> It(Function); It && It->HasAnyPropertyFlags(CPF_Parm); ++It)
        {
            It->DestroyValue_InContainer(Params);
        }
    }
    FMemory::Free(Params);
}

static ERustealErrorCode CallFunctionImpl(RustealUObjectHandle Obj, RustealUFunctionHandle Func, uint8* Params)
{
    UObject* Object = static_cast<UObject*>(Obj.ptr);
    if (!::IsValid(Object)) return ERustealErrorCode::ObjectDestroyed;
    UFunction* Function = static_cast<UFunction*>(Func.ptr);
    if (!Function) return ERustealErrorCode::FunctionNotFound;
    Object->ProcessEvent(Function, Params);
    return ERustealErrorCode::Ok;
}

static RustealFPropertyHandle GetFunctionParamImpl(RustealUFunctionHandle Func, const uint8* Name, uint32 NameLen)
{
    UFunction* Function = static_cast<UFunction*>(Func.ptr);
    if (!Function) return RustealFPropertyHandle{ nullptr };
    const FName PropName = Utf8ToFName(Name, NameLen);
    FProperty* Prop = Function->FindPropertyByName(PropName);
    return RustealFPropertyHandle{ Prop };
}

static uint32 GetPropertyOffsetImpl(RustealFPropertyHandle Prop)
{
    FProperty* Property = static_cast<FProperty*>(Prop.ptr);
    if (!Property) return 0;
    return static_cast<uint32>(Property->GetOffset_ForUFunction());
}

static RustealUFunctionHandle FindFunctionByClassImpl(RustealUClassHandle Cls, const uint8* Name, uint32 NameLen)
{
    UClass* Class = static_cast<UClass*>(Cls.ptr);
    if (!Class) return RustealUFunctionHandle{ nullptr };
    const FName FuncName = Utf8ToFName(Name, NameLen);
    UFunction* Func = Class->FindFunctionByName(FuncName);
    return RustealUFunctionHandle{ Func };
}

static uint32 GetElementSizeImpl(RustealFPropertyHandle Prop)
{
    FProperty* Property = static_cast<FProperty*>(Prop.ptr);
    return Property ? Property->GetElementSize() : 0;
}

static uint32 GetStructSizeImpl(RustealUStructHandle UStruct)
{
    UScriptStruct* Struct = static_cast<UScriptStruct*>(UStruct.ptr);
    return Struct ? static_cast<uint32>(Struct->GetStructureSize()) : 0;
}

static ERustealErrorCode InitializeStructImpl(RustealUStructHandle UStructHandle, uint8* Data)
{
    UScriptStruct* Struct = static_cast<UScriptStruct*>(UStructHandle.ptr);
    if (!Struct || !Data) return ERustealErrorCode::NullArgument;
    Struct->InitializeStruct(Data);
    return ERustealErrorCode::Ok;
}

static ERustealErrorCode DestroyStructImpl(RustealUStructHandle UStructHandle, uint8* Data)
{
    UScriptStruct* Struct = static_cast<UScriptStruct*>(UStructHandle.ptr);
    if (!Struct || !Data) return ERustealErrorCode::NullArgument;
    Struct->DestroyStruct(Data);
    return ERustealErrorCode::Ok;
}

// ---------------------------------------------------------------------------
// Static instance
// ---------------------------------------------------------------------------

FRustealReflectionApi GReflectionApi = {
    &FindClassImpl,
    &FindPropertyImpl,
    &GetStaticClassImpl,
    &GetPropertySizeImpl,
    &FindStructImpl,
    &FindStructPropertyImpl,
    &FindFunctionImpl,
    &AllocParamsImpl,
    &FreeParamsImpl,
    &CallFunctionImpl,
    &GetFunctionParamImpl,
    &GetPropertyOffsetImpl,
    &FindFunctionByClassImpl,
    &GetElementSizeImpl,
    &GetStructSizeImpl,
    &InitializeStructImpl,
    &DestroyStructImpl,
};
