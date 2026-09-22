// RustealReifyApiImpl.cpp — FRustealReifyApi implementation + UObject delete listener.

#include "RustealApiTable.h"
#include "URustealReifiedClass.h"
#include "URustealReifiedFunction.h"
#include "RustealModule.h"
#include "Engine/Blueprint.h"
#include "GameFramework/Actor.h"
#include "UObject/UObjectGlobals.h"
#include "UObject/UnrealType.h"
#include "UObject/UObjectArray.h"
#include "UObject/UObjectIterator.h"

// Helper: convert UTF-8 byte slice to FName.
static FName ReifyUtf8ToFName(const uint8* Name, uint32 NameLen)
{
    const FString Str(NameLen, UTF8_TO_TCHAR(reinterpret_cast<const char*>(Name)));
    return FName(*Str);
}

// Helper: convert UTF-8 byte slice to FString.
static FString ReifyUtf8ToFString(const uint8* Name, uint32 NameLen)
{
    return FString(NameLen, UTF8_TO_TCHAR(reinterpret_cast<const char*>(Name)));
}

// ---------------------------------------------------------------------------
// Helper: Create an FProperty by type enum
// ---------------------------------------------------------------------------

static FProperty* CreatePropertyByType(
    FFieldVariant Owner,
    FName PropName,
    ERustealReifyPropType PropType,
    const FRustealReifyPropExtra* Extra)
{
    FProperty* Prop = nullptr;

    switch (PropType)
    {
    case ERustealReifyPropType::Bool:
    {
        FBoolProperty* BoolProp = new FBoolProperty(Owner, PropName, RF_Public);
        Prop = BoolProp;
        break;
    }
    case ERustealReifyPropType::Int8:
    {
        Prop = new FInt8Property(Owner, PropName, RF_Public);
        break;
    }
    case ERustealReifyPropType::Int16:
    {
        Prop = new FInt16Property(Owner, PropName, RF_Public);
        break;
    }
    case ERustealReifyPropType::Int32:
    {
        Prop = new FIntProperty(Owner, PropName, RF_Public);
        break;
    }
    case ERustealReifyPropType::Int64:
    {
        Prop = new FInt64Property(Owner, PropName, RF_Public);
        break;
    }
    case ERustealReifyPropType::UInt8:
    {
        Prop = new FByteProperty(Owner, PropName, RF_Public);
        break;
    }
    case ERustealReifyPropType::UInt16:
    {
        Prop = new FUInt16Property(Owner, PropName, RF_Public);
        break;
    }
    case ERustealReifyPropType::UInt32:
    {
        Prop = new FUInt32Property(Owner, PropName, RF_Public);
        break;
    }
    case ERustealReifyPropType::UInt64:
    {
        Prop = new FUInt64Property(Owner, PropName, RF_Public);
        break;
    }
    case ERustealReifyPropType::Float:
    {
        Prop = new FFloatProperty(Owner, PropName, RF_Public);
        break;
    }
    case ERustealReifyPropType::Double:
    {
        Prop = new FDoubleProperty(Owner, PropName, RF_Public);
        break;
    }
    case ERustealReifyPropType::String:
    {
        Prop = new FStrProperty(Owner, PropName, RF_Public);
        break;
    }
    case ERustealReifyPropType::Name:
    {
        Prop = new FNameProperty(Owner, PropName, RF_Public);
        break;
    }
    case ERustealReifyPropType::Text:
    {
        Prop = new FTextProperty(Owner, PropName, RF_Public);
        break;
    }
    case ERustealReifyPropType::Object:
    {
        FObjectProperty* ObjProp = new FObjectProperty(Owner, PropName, RF_Public);
        if (Extra && Extra->class_handle.ptr)
        {
            ObjProp->PropertyClass = static_cast<UClass*>(Extra->class_handle.ptr);
        }
        else
        {
            ObjProp->PropertyClass = UObject::StaticClass();
        }
        Prop = ObjProp;
        break;
    }
    case ERustealReifyPropType::Class:
    {
        FClassProperty* ClsProp = new FClassProperty(Owner, PropName, RF_Public);
        if (Extra)
        {
            ClsProp->PropertyClass = Extra->class_handle.ptr
                ? static_cast<UClass*>(Extra->class_handle.ptr)
                : UObject::StaticClass();
            ClsProp->MetaClass = Extra->meta_class_handle.ptr
                ? static_cast<UClass*>(Extra->meta_class_handle.ptr)
                : UObject::StaticClass();
        }
        else
        {
            ClsProp->PropertyClass = UObject::StaticClass();
            ClsProp->MetaClass = UObject::StaticClass();
        }
        Prop = ClsProp;
        break;
    }
    case ERustealReifyPropType::Struct:
    {
        UScriptStruct* ScriptStruct = (Extra && Extra->struct_handle.ptr)
            ? static_cast<UScriptStruct*>(Extra->struct_handle.ptr)
            : nullptr;
        if (!ScriptStruct)
        {
            UE_LOG(LogRusteal, Error, TEXT("[Rusteal] CreatePropertyByType(Struct): null struct_handle"));
            return nullptr;
        }
        FStructProperty* StructProp = new FStructProperty(Owner, PropName, RF_Public);
        StructProp->Struct = ScriptStruct;
        Prop = StructProp;
        break;
    }
    case ERustealReifyPropType::Enum:
    {
        UEnum* EnumType = (Extra && Extra->enum_handle.ptr)
            ? static_cast<UEnum*>(Extra->enum_handle.ptr)
            : nullptr;
        if (!EnumType)
        {
            UE_LOG(LogRusteal, Error, TEXT("[Rusteal] CreatePropertyByType(Enum): null enum_handle"));
            return nullptr;
        }
        FEnumProperty* EnumProp = new FEnumProperty(Owner, PropName, RF_Public);
        EnumProp->SetEnum(EnumType);
        // Create the underlying numeric property
        FNumericProperty* UnderlyingProp = new FByteProperty(EnumProp, TEXT("UnderlyingType"), RF_Public);
        EnumProp->AddCppProperty(UnderlyingProp);
        Prop = EnumProp;
        break;
    }
    default:
        UE_LOG(LogRusteal, Error, TEXT("[Rusteal] CreatePropertyByType: unknown type %d"), static_cast<int>(PropType));
        return nullptr;
    }

    return Prop;
}

// ---------------------------------------------------------------------------
// Helper: Copy parameter properties from a parent function to a new override
// ---------------------------------------------------------------------------

static void CopyParamsFromParentFunction(UFunction* NewFunc, UFunction* ParentFunc)
{
    for (FField* SrcField = ParentFunc->ChildProperties; SrcField; SrcField = SrcField->Next)
    {
        FProperty* SrcProp = CastField<FProperty>(SrcField);
        if (!SrcProp)
        {
            continue;
        }

        // Map source property type back to our enum so we can use CreatePropertyByType
        ERustealReifyPropType PropType;
        FRustealReifyPropExtra Extra = {};

        // Check FClassProperty before FObjectProperty (FClassProperty extends FObjectProperty)
        if (SrcProp->IsA<FBoolProperty>())
        {
            PropType = ERustealReifyPropType::Bool;
        }
        else if (SrcProp->IsA<FInt8Property>())
        {
            PropType = ERustealReifyPropType::Int8;
        }
        else if (SrcProp->IsA<FInt16Property>())
        {
            PropType = ERustealReifyPropType::Int16;
        }
        else if (SrcProp->IsA<FIntProperty>())
        {
            PropType = ERustealReifyPropType::Int32;
        }
        else if (SrcProp->IsA<FInt64Property>())
        {
            PropType = ERustealReifyPropType::Int64;
        }
        else if (SrcProp->IsA<FByteProperty>())
        {
            PropType = ERustealReifyPropType::UInt8;
        }
        else if (SrcProp->IsA<FUInt16Property>())
        {
            PropType = ERustealReifyPropType::UInt16;
        }
        else if (SrcProp->IsA<FUInt32Property>())
        {
            PropType = ERustealReifyPropType::UInt32;
        }
        else if (SrcProp->IsA<FUInt64Property>())
        {
            PropType = ERustealReifyPropType::UInt64;
        }
        else if (SrcProp->IsA<FFloatProperty>())
        {
            PropType = ERustealReifyPropType::Float;
        }
        else if (SrcProp->IsA<FDoubleProperty>())
        {
            PropType = ERustealReifyPropType::Double;
        }
        else if (SrcProp->IsA<FStrProperty>())
        {
            PropType = ERustealReifyPropType::String;
        }
        else if (SrcProp->IsA<FNameProperty>())
        {
            PropType = ERustealReifyPropType::Name;
        }
        else if (SrcProp->IsA<FTextProperty>())
        {
            PropType = ERustealReifyPropType::Text;
        }
        else if (SrcProp->IsA<FStructProperty>())
        {
            PropType = ERustealReifyPropType::Struct;
            Extra.struct_handle.ptr = CastField<FStructProperty>(SrcProp)->Struct;
        }
        else if (SrcProp->IsA<FClassProperty>())
        {
            PropType = ERustealReifyPropType::Class;
            Extra.class_handle.ptr = CastField<FClassProperty>(SrcProp)->PropertyClass;
            Extra.meta_class_handle.ptr = CastField<FClassProperty>(SrcProp)->MetaClass;
        }
        else if (SrcProp->IsA<FObjectProperty>())
        {
            PropType = ERustealReifyPropType::Object;
            Extra.class_handle.ptr = CastField<FObjectProperty>(SrcProp)->PropertyClass;
        }
        else if (SrcProp->IsA<FEnumProperty>())
        {
            PropType = ERustealReifyPropType::Enum;
            Extra.enum_handle.ptr = CastField<FEnumProperty>(SrcProp)->GetEnum();
        }
        else
        {
            UE_LOG(LogRusteal, Warning,
                TEXT("[Rusteal] CopyParamsFromParent: unsupported property type '%s' for param '%s', skipping"),
                *SrcProp->GetClass()->GetName(), *SrcProp->GetName());
            continue;
        }

        FProperty* NewProp = CreatePropertyByType(
            FFieldVariant(NewFunc), SrcProp->GetFName(), PropType, &Extra);

        if (NewProp)
        {
            NewProp->PropertyFlags = SrcProp->PropertyFlags;

            // Append to end of ChildProperties (preserve parameter order)
            if (!NewFunc->ChildProperties)
            {
                NewFunc->ChildProperties = NewProp;
            }
            else
            {
                FField* Last = NewFunc->ChildProperties;
                while (Last->Next)
                {
                    Last = Last->Next;
                }
                Last->Next = NewProp;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// API implementations
// ---------------------------------------------------------------------------

// Shared package pointer for all reified classes.
static UPackage* GRustealReifyPackage = nullptr;

static UPackage* GetOrCreateRustealPackage()
{
    if (!GRustealReifyPackage)
    {
        GRustealReifyPackage = CreatePackage(TEXT("/Script/Rusteal"));
        GRustealReifyPackage->SetPackageFlags(PKG_CompiledIn);
    }
    return GRustealReifyPackage;
}

static RustealUClassHandle CreateClassImpl(
    const uint8* Name, uint32 NameLen,
    RustealUClassHandle Parent,
    uint64 RustTypeId)
{
    UClass* ParentClass = static_cast<UClass*>(Parent.ptr);
    if (!ParentClass)
    {
        UE_LOG(LogRusteal, Error, TEXT("[Rusteal] CreateClass: null parent class"));
        return RustealUClassHandle{ nullptr };
    }

    const FString ClassName = ReifyUtf8ToFString(Name, NameLen);

    // --- Hot reload path: if a class with this name already exists, reuse it ---
    UPackage* RustealPackage = GetOrCreateRustealPackage();
    URustealReifiedClass* Existing = FindObject<URustealReifiedClass>(
        RustealPackage, *ClassName);
    if (Existing)
    {
        // Update the Rust type ID (may have changed if Rust struct layout changed).
        Existing->RustTypeId = RustTypeId;

        UE_LOG(LogRusteal, Display,
            TEXT("[Rusteal] Hot reload: reusing existing class %s (type_id: %llu)"),
            *ClassName, RustTypeId);

        return RustealUClassHandle{ Existing };
    }

    // --- Normal path: create new class ---
    URustealReifiedClass* NewClass = NewObject<URustealReifiedClass>(
        RustealPackage,
        FName(*ClassName),
        RF_Public | RF_Standalone);

    NewClass->RustTypeId = RustTypeId;

    // Walk up to find the native (C++) superclass.
    UClass* NativeSuper = ParentClass;
    while (NativeSuper && !NativeSuper->HasAnyClassFlags(CLASS_Native))
    {
        NativeSuper = NativeSuper->GetSuperClass();
    }
    NewClass->NativeSuperClass = NativeSuper ? NativeSuper : ParentClass;

    // Set up class hierarchy.
    NewClass->SetSuperStruct(ParentClass);
    NewClass->ClassConstructor = &URustealReifiedClass::RustealClassConstructor;

    // Propagate inheritable flags from parent (CLASS_HasInstancedReference, etc.).
    // Bind() propagates ClassCastFlags but NOT CLASS_Inherit flags.
    // Exclude config-related flags: dynamically-created classes don't have a
    // ClassConfigName and would crash in GetConfigName()/LoadConfig().
    constexpr EClassFlags ConfigRelatedFlags = EClassFlags(
        CLASS_Config | CLASS_DefaultConfig | CLASS_PerObjectConfig |
        CLASS_ConfigDoNotCheckDefaults | CLASS_GlobalUserConfig |
        CLASS_ProjectUserConfig | CLASS_PerPlatformConfig);
    NewClass->ClassFlags |= (ParentClass->ClassFlags & CLASS_Inherit & ~ConfigRelatedFlags) | CLASS_CompiledFromBlueprint;

    // Create a stub UBlueprint so that FBlueprintActionDatabase registers
    // our functions.  Without this, the action database sees our class as a
    // UBlueprintGeneratedClass with null ClassGeneratedBy and skips it.
    UBlueprint* StubBP = NewObject<UBlueprint>(
        RustealPackage, FName(*(ClassName + TEXT("_BP"))),
        RF_Public | RF_Standalone);
    StubBP->GeneratedClass = NewClass;
    StubBP->SkeletonGeneratedClass = NewClass;
    StubBP->ParentClass = ParentClass;
    StubBP->BlueprintType = BPTYPE_Normal;
    StubBP->Status = BS_UpToDate;
    StubBP->AddToRoot();
    NewClass->ClassGeneratedBy = StubBP;

#if WITH_EDITORONLY_DATA
    // Mark as "cooked" so GetGeneratedClassesHierarchy skips the
    // BS_Error check (our stub UBlueprint is always up-to-date).
    NewClass->bCooked = true;
#endif

    // Prevent garbage collection.
    NewClass->AddToRoot();

    UE_LOG(LogRusteal, Display, TEXT("[Rusteal] Created reified class: %s (parent: %s, type_id: %llu)"),
        *ClassName, *ParentClass->GetName(), RustTypeId);

    return RustealUClassHandle{ NewClass };
}

static RustealFPropertyHandle AddPropertyImpl(
    RustealUClassHandle Cls,
    const uint8* Name, uint32 NameLen,
    uint32 PropType, uint64 PropFlags,
    const FRustealReifyPropExtra* Extra)
{
    UClass* Class = static_cast<UClass*>(Cls.ptr);
    if (!Class)
    {
        return RustealFPropertyHandle{ nullptr };
    }

    const FName PropName = ReifyUtf8ToFName(Name, NameLen);

    // --- Hot reload path: if a property with this name already exists, reuse it ---
    for (FProperty* P = Class->PropertyLink; P; P = P->PropertyLinkNext)
    {
        if (P->GetOwnerClass() == Class && P->GetFName() == PropName)
        {
            UE_LOG(LogRusteal, Display,
                TEXT("[Rusteal] Hot reload: reusing existing property %s::%s"),
                *Class->GetName(), *PropName.ToString());
            return RustealFPropertyHandle{ P };
        }
    }

    // --- Normal path: create new property ---
    FProperty* Prop = CreatePropertyByType(
        FFieldVariant(Class),
        PropName,
        static_cast<ERustealReifyPropType>(PropType),
        Extra);

    if (!Prop)
    {
        return RustealFPropertyHandle{ nullptr };
    }

    Prop->PropertyFlags |= static_cast<EPropertyFlags>(PropFlags);
    Class->AddCppProperty(Prop);

    return RustealFPropertyHandle{ Prop };
}

static RustealUFunctionHandle AddFunctionImpl(
    RustealUClassHandle Cls,
    const uint8* Name, uint32 NameLen,
    uint64 CallbackId, uint32 FuncFlags)
{
    UClass* Class = static_cast<UClass*>(Cls.ptr);
    if (!Class)
    {
        return RustealUFunctionHandle{ nullptr };
    }

    const FString FuncName = ReifyUtf8ToFString(Name, NameLen);

    // --- Hot reload path: if this function already exists, just update the callback ID ---
    UFunction* ExistingFunc = Class->FindFunctionByName(FName(*FuncName));
    if (ExistingFunc)
    {
        if (URustealReifiedFunction* Reified = Cast<URustealReifiedFunction>(ExistingFunc))
        {
            Reified->CallbackId = CallbackId;
            UE_LOG(LogRusteal, Display,
                TEXT("[Rusteal] Hot reload: updated CallbackId for %s::%s (id: %llu)"),
                *Class->GetName(), *FuncName, CallbackId);
            return RustealUFunctionHandle{ ExistingFunc };
        }
    }

    // --- Normal path: create new function ---
    URustealReifiedFunction* NewFunc = NewObject<URustealReifiedFunction>(
        Class, FName(*FuncName), RF_Public | RF_MarkAsNative);

    NewFunc->CallbackId = CallbackId;
    NewFunc->FunctionFlags = static_cast<EFunctionFlags>(FuncFlags) | FUNC_Native;

    // Set the native function pointer to the thunk.
    NewFunc->SetNativeFunc(&URustealReifiedFunction::execCallRustFunction);

    // Link into the class's Children list so TFieldIterator<UFunction> can
    // discover it (used by Blueprint action menu, StaticLink, etc.).
    NewFunc->Next = Class->Children;
    Class->Children = NewFunc;

    // For Override functions (BlueprintEvent), copy parameter definitions from
    // the parent class's function. This way the macro doesn't need to know how
    // to register struct/complex parameter types — they're inherited from UHT.
    if (static_cast<EFunctionFlags>(FuncFlags) & FUNC_BlueprintEvent)
    {
        UFunction* ParentFunc = Class->GetSuperClass()
            ? Class->GetSuperClass()->FindFunctionByName(NewFunc->GetFName())
            : nullptr;

        if (ParentFunc)
        {
            CopyParamsFromParentFunction(NewFunc, ParentFunc);
            UE_LOG(LogRusteal, Display,
                TEXT("[Rusteal] Override %s::%s: copied params from parent %s"),
                *Class->GetName(), *FuncName, *ParentFunc->GetOuter()->GetName());
        }
    }

    // Register the native function name for the VM.
    Class->AddNativeFunction(*FuncName, &URustealReifiedFunction::execCallRustFunction);
    Class->AddFunctionToFunctionMap(NewFunc, NewFunc->GetFName());

    return RustealUFunctionHandle{ NewFunc };
}

static ERustealErrorCode AddFunctionParamImpl(
    RustealUFunctionHandle Func,
    const uint8* Name, uint32 NameLen,
    uint32 PropType, uint64 ParamFlags,
    const FRustealReifyPropExtra* Extra)
{
    URustealReifiedFunction* Function = Cast<URustealReifiedFunction>(static_cast<UFunction*>(Func.ptr));
    if (!Function)
    {
        return ERustealErrorCode::NullArgument;
    }

    const FName ParamName = ReifyUtf8ToFName(Name, NameLen);

    // --- Hot reload path: if a param with this name already exists, reuse it ---
    for (FField* Field = Function->ChildProperties; Field; Field = Field->Next)
    {
        if (FProperty* Existing = CastField<FProperty>(Field))
        {
            if (Existing->GetFName() == ParamName)
            {
                return ERustealErrorCode::Ok;
            }
        }
    }

    // --- Normal path: create new parameter property ---
    FProperty* Param = CreatePropertyByType(
        FFieldVariant(Function),
        ParamName,
        static_cast<ERustealReifyPropType>(PropType),
        Extra);

    if (!Param)
    {
        return ERustealErrorCode::InternalError;
    }

    // Set parameter flags (CPF_Parm must always be set for function parameters).
    Param->PropertyFlags |= static_cast<EPropertyFlags>(ParamFlags) | CPF_Parm;

    // Append to the END of ChildProperties instead of using AddCppProperty
    // (which prepends). This keeps parameters in declaration order, matching
    // UHT convention. The Blueprint compiler, bytecode VM, and our thunk all
    // iterate ChildProperties in linked-list order, so they must agree.
    if (Function->ChildProperties == nullptr)
    {
        Function->ChildProperties = Param;
    }
    else
    {
        FField* Last = Function->ChildProperties;
        while (Last->Next)
        {
            Last = Last->Next;
        }
        Last->Next = Param;
    }

    return ERustealErrorCode::Ok;
}

static ERustealErrorCode FinalizeClassImpl(RustealUClassHandle Cls)
{
    URustealReifiedClass* Class = Cast<URustealReifiedClass>(static_cast<UClass*>(Cls.ptr));
    if (!Class)
    {
        return ERustealErrorCode::NullArgument;
    }

    // Hot reload path: if already finalized (Bind/StaticLink done), skip.
    if (Class->HasAnyClassFlags(CLASS_Constructed))
    {
        UE_LOG(LogRusteal, Display,
            TEXT("[Rusteal] Hot reload: class %s already finalized, skipping"),
            *Class->GetName());
        return ERustealErrorCode::Ok;
    }

    // Finalize function parameter layouts: Bind → StaticLink each function.
    for (TFieldIterator<UFunction> FuncIt(Class, EFieldIteratorFlags::ExcludeSuper); FuncIt; ++FuncIt)
    {
        UFunction* Func = *FuncIt;
        Func->Bind();
        Func->StaticLink(true);
    }

    // Finalize the class itself.
    Class->Bind();
    Class->StaticLink(true);

    // Build the GC reference token stream so the garbage collector can
    // properly trace UObject* references within instances of this class.
    Class->AssembleReferenceTokenStream(true);

    // Force CDO creation and run BPGC post-load initialization
    // (builds CustomPropertyListForPostConstruction, etc.).
    UObject* CDO = Class->GetDefaultObject(true);
    Class->PostLoadDefaultObject(CDO);

    // Enable ticking if this class overrides ReceiveTick.
    // Some native classes (e.g. AGameModeBase) set bCanEverTick=false in their
    // C++ constructor, which makes SetActorTickEnabled(true) silently fail.
    if (AActor* ActorCDO = Cast<AActor>(CDO))
    {
        for (TFieldIterator<UFunction> FuncIt(Class, EFieldIteratorFlags::ExcludeSuper); FuncIt; ++FuncIt)
        {
            if ((*FuncIt)->GetFName() == FName(TEXT("ReceiveTick")))
            {
                ActorCDO->PrimaryActorTick.bCanEverTick = true;
                ActorCDO->PrimaryActorTick.bStartWithTickEnabled = true;
                UE_LOG(LogRusteal, Display,
                    TEXT("[Rusteal] Enabled tick for %s (ReceiveTick override detected)"),
                    *Class->GetName());
                break;
            }
        }
    }

    UE_LOG(LogRusteal, Display, TEXT("[Rusteal] Finalized reified class: %s (size: %d, super_size: %d)"),
        *Class->GetName(), Class->GetPropertiesSize(),
        Class->GetSuperClass() ? Class->GetSuperClass()->GetPropertiesSize() : 0);

    // Validate property chain integrity.
    int32 PropCount = 0;
    for (FProperty* P = Class->PropertyLink; P; P = P->PropertyLinkNext)
    {
        if (P->GetOwnerClass() == Class)
        {
            UE_LOG(LogRusteal, Display, TEXT("[Rusteal]   Property: %s offset=%d size=%d"),
                *P->GetName(), P->GetOffset_ForInternal(), P->GetSize());
        }
        PropCount++;
        if (PropCount > 10000)
        {
            UE_LOG(LogRusteal, Error, TEXT("[Rusteal] PropertyLink chain appears corrupt (>10000 entries)"));
            break;
        }
    }
    UE_LOG(LogRusteal, Display, TEXT("[Rusteal]   Total properties in chain: %d"), PropCount);

    return ERustealErrorCode::Ok;
}

static RustealUObjectHandle GetCdoImpl(RustealUClassHandle Cls)
{
    UClass* Class = static_cast<UClass*>(Cls.ptr);
    if (!Class)
    {
        return RustealUObjectHandle{ nullptr };
    }
    return RustealUObjectHandle{ Class->GetDefaultObject() };
}

// ---------------------------------------------------------------------------
// Default subobject registration
// ---------------------------------------------------------------------------

static ERustealErrorCode AddDefaultSubobjectImpl(
    RustealUClassHandle Cls,
    const uint8* Name, uint32 NameLen,
    RustealUClassHandle CompClass,
    uint32 Flags,
    const uint8* AttachParent, uint32 AttachLen)
{
    URustealReifiedClass* RC = Cast<URustealReifiedClass>(static_cast<UClass*>(Cls.ptr));
    if (!RC) return ERustealErrorCode::InvalidCast;

    UClass* CompUClass = static_cast<UClass*>(CompClass.ptr);
    if (!CompUClass) return ERustealErrorCode::PropertyNotFound;

    FRustealComponentDef Def;
    Def.SubobjectName = FName(ReifyUtf8ToFString(Name, NameLen));
    Def.ComponentClass = CompUClass;
    Def.bIsRoot = (Flags & 1) != 0;
    Def.bIsTransient = (Flags & 2) != 0;
    Def.AttachParentName = AttachLen > 0
        ? FName(ReifyUtf8ToFString(AttachParent, AttachLen))
        : NAME_None;

    // Hot reload: avoid duplicate defs
    RC->ComponentDefs.RemoveAll([&](const FRustealComponentDef& D) {
        return D.SubobjectName == Def.SubobjectName;
    });
    RC->ComponentDefs.Add(MoveTemp(Def));

    UE_LOG(LogRusteal, Display, TEXT("[Rusteal] Registered default subobject '%s' (class: %s) on %s"),
        *Def.SubobjectName.ToString(), *CompUClass->GetName(), *RC->GetName());

    return ERustealErrorCode::Ok;
}

static RustealUObjectHandle FindDefaultSubobjectImpl(
    RustealUObjectHandle Owner,
    const uint8* Name, uint32 NameLen)
{
    UObject* Obj = static_cast<UObject*>(Owner.ptr);
    if (!Obj) return RustealUObjectHandle{ nullptr };

    FName SubName(ReifyUtf8ToFString(Name, NameLen));
    UObject* Sub = Obj->GetDefaultSubobjectByName(SubName);
    return RustealUObjectHandle{ Sub };
}

// ---------------------------------------------------------------------------
// FRustealDeleteListener — Notifies Rust when a reified-class instance is GC'd
// ---------------------------------------------------------------------------

class FRustealDeleteListener : public FUObjectArray::FUObjectDeleteListener
{
public:
    virtual void NotifyUObjectDeleted(const UObjectBase* Object, int32 Index) override
    {
        // Only handle objects whose class is a reified class.
        const UClass* ObjClass = Object->GetClass();
        const URustealReifiedClass* ReifiedClass = Cast<URustealReifiedClass>(ObjClass);
        if (!ReifiedClass)
        {
            return;
        }

        const FRustealRustCallbacks* Callbacks = GetRustealRustCallbacks();
        if (Callbacks && Callbacks->drop_rust_instance)
        {
            Callbacks->drop_rust_instance(
                RustealUObjectHandle{ const_cast<UObjectBase*>(Object) },
                ReifiedClass->RustTypeId,
                nullptr);
        }
    }

    virtual void OnUObjectArrayShutdown() override
    {
        GUObjectArray.RemoveUObjectDeleteListener(this);
    }
};

static FRustealDeleteListener GDeleteListener;

void RustealReifyRegisterDeleteListener()
{
    GUObjectArray.AddUObjectDeleteListener(&GDeleteListener);
}

void RustealReifyUnregisterDeleteListener()
{
    GUObjectArray.RemoveUObjectDeleteListener(&GDeleteListener);
}

// ---------------------------------------------------------------------------
// Hot reload helpers (called from RustealModule.cpp)
// ---------------------------------------------------------------------------

void RustealReifyForEachReifiedInstance(
    TFunctionRef<void(UObject*, URustealReifiedClass*)> Callback)
{
    for (FThreadSafeObjectIterator It; It; ++It)
    {
        UObject* Obj = static_cast<UObject*>(*It);
        // Skip CDOs — they don't have meaningful Rust instance data.
        if (Obj->HasAnyFlags(RF_ClassDefaultObject))
        {
            continue;
        }
        URustealReifiedClass* ReifiedClass = Cast<URustealReifiedClass>(Obj->GetClass());
        if (ReifiedClass)
        {
            Callback(Obj, ReifiedClass);
        }
    }
}

// ---------------------------------------------------------------------------
// Export the API table
// ---------------------------------------------------------------------------

FRustealReifyApi GReifyApi = {
    &CreateClassImpl,
    &AddPropertyImpl,
    &AddFunctionImpl,
    &AddFunctionParamImpl,
    &FinalizeClassImpl,
    &GetCdoImpl,
    &AddDefaultSubobjectImpl,
    &FindDefaultSubobjectImpl,
};
