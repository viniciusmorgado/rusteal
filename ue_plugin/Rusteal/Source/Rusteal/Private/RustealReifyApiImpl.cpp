#include "Engine/Blueprint.h"
#include "GameFramework/Actor.h"
#include "Engine/Engine.h"
#include "Misc/CoreDelegates.h"
#include "Serialization/AsyncLoadingEvents.h"
#include "RustealApiTable.h"
#include "RustealLibraries.h"
#include "RustealLibrary.h"
#include "RustealModule.h"
#include "UObject/UObjectArray.h"
#include "UObject/UObjectGlobals.h"
#include "UObject/UObjectIterator.h"
#include "UObject/UnrealType.h"
#include "URustealReifiedClass.h"
#include "URustealReifiedFunction.h"

static FName ReifyUtf8ToFName(const uint8 *Name, uint32 NameLen) {
  const FString Str(NameLen,
                    UTF8_TO_TCHAR(reinterpret_cast<const char *>(Name)));

  return FName(*Str);
}

static FString ReifyUtf8ToFString(const uint8 *Name, uint32 NameLen) {
  return FString(NameLen, UTF8_TO_TCHAR(reinterpret_cast<const char *>(Name)));
}

static FProperty *CreatePropertyByType(FFieldVariant Owner, FName PropName,
                                       ERustealReifyPropType PropType,
                                       const FRustealReifyPropExtra *Extra) {
  FProperty *Prop = nullptr;

  switch (PropType) {
  case ERustealReifyPropType::Bool: {
    FBoolProperty *BoolProp = new FBoolProperty(Owner, PropName, RF_Public);
    Prop = BoolProp;
    break;
  }
  case ERustealReifyPropType::Int8: {
    Prop = new FInt8Property(Owner, PropName, RF_Public);
    break;
  }
  case ERustealReifyPropType::Int16: {
    Prop = new FInt16Property(Owner, PropName, RF_Public);
    break;
  }
  case ERustealReifyPropType::Int32: {
    Prop = new FIntProperty(Owner, PropName, RF_Public);
    break;
  }
  case ERustealReifyPropType::Int64: {
    Prop = new FInt64Property(Owner, PropName, RF_Public);
    break;
  }
  case ERustealReifyPropType::UInt8: {
    Prop = new FByteProperty(Owner, PropName, RF_Public);
    break;
  }
  case ERustealReifyPropType::UInt16: {
    Prop = new FUInt16Property(Owner, PropName, RF_Public);
    break;
  }
  case ERustealReifyPropType::UInt32: {
    Prop = new FUInt32Property(Owner, PropName, RF_Public);
    break;
  }
  case ERustealReifyPropType::UInt64: {
    Prop = new FUInt64Property(Owner, PropName, RF_Public);
    break;
  }
  case ERustealReifyPropType::Float: {
    Prop = new FFloatProperty(Owner, PropName, RF_Public);
    break;
  }
  case ERustealReifyPropType::Double: {
    Prop = new FDoubleProperty(Owner, PropName, RF_Public);
    break;
  }
  case ERustealReifyPropType::String: {
    Prop = new FStrProperty(Owner, PropName, RF_Public);
    break;
  }
  case ERustealReifyPropType::Name: {
    Prop = new FNameProperty(Owner, PropName, RF_Public);
    break;
  }
  case ERustealReifyPropType::Text: {
    Prop = new FTextProperty(Owner, PropName, RF_Public);
    break;
  }
  case ERustealReifyPropType::Object: {
    FObjectProperty *ObjProp = new FObjectProperty(Owner, PropName, RF_Public);
    if (Extra && Extra->class_handle.ptr) {
      ObjProp->PropertyClass = static_cast<UClass *>(Extra->class_handle.ptr);
    } else {
      ObjProp->PropertyClass = UObject::StaticClass();
    }
    Prop = ObjProp;
    break;
  }
  case ERustealReifyPropType::Class: {
    FClassProperty *ClsProp = new FClassProperty(Owner, PropName, RF_Public);
    if (Extra) {
      ClsProp->PropertyClass =
          Extra->class_handle.ptr
              ? static_cast<UClass *>(Extra->class_handle.ptr)
              : UClass::StaticClass();

      ClsProp->MetaClass =
          Extra->meta_class_handle.ptr
              ? static_cast<UClass *>(Extra->meta_class_handle.ptr)
              : UObject::StaticClass();
    } else {
      ClsProp->PropertyClass = UClass::StaticClass();
      ClsProp->MetaClass = UObject::StaticClass();
    }
    Prop = ClsProp;
    break;
  }
  case ERustealReifyPropType::Struct: {
    UScriptStruct *ScriptStruct =
        (Extra && Extra->struct_handle.ptr)
            ? static_cast<UScriptStruct *>(Extra->struct_handle.ptr)
            : nullptr;
    if (!ScriptStruct) {
      UE_LOG(
          LogRusteal, Error,
          TEXT("[Rusteal] CreatePropertyByType(Struct): null struct_handle"));

      return nullptr;
    }
    FStructProperty *StructProp =
        new FStructProperty(Owner, PropName, RF_Public);
    StructProp->Struct = ScriptStruct;
    Prop = StructProp;
    break;
  }
  case ERustealReifyPropType::Enum: {
    UEnum *EnumType = (Extra && Extra->enum_handle.ptr)
                          ? static_cast<UEnum *>(Extra->enum_handle.ptr)
                          : nullptr;
    if (!EnumType) {
      UE_LOG(LogRusteal, Error,
             TEXT("[Rusteal] CreatePropertyByType(Enum): null enum_handle"));

      return nullptr;
    }
    FEnumProperty *EnumProp = new FEnumProperty(Owner, PropName, RF_Public);
    EnumProp->SetEnum(EnumType);
    FNumericProperty *UnderlyingProp =
        new FByteProperty(EnumProp, TEXT("UnderlyingType"), RF_Public);
    EnumProp->AddCppProperty(UnderlyingProp);
    Prop = EnumProp;
    break;
  }
  case ERustealReifyPropType::Array: {
    const ERustealReifyPropType InnerType =
        Extra ? static_cast<ERustealReifyPropType>(Extra->inner_prop_type)
              : ERustealReifyPropType::Array;
    if (InnerType == ERustealReifyPropType::Array) {
      UE_LOG(LogRusteal, Error,
             TEXT("[Rusteal] CreatePropertyByType(Array): missing or nested "
                  "element type for %s"),
             *PropName.ToString());

      return nullptr;
    }
    FArrayProperty *ArrayProp = new FArrayProperty(Owner, PropName);
    FProperty *Inner = CreatePropertyByType(FFieldVariant(ArrayProp), PropName,
                                            InnerType, Extra);
    if (!Inner) {
      delete ArrayProp;
      return nullptr;
    }
    ArrayProp->AddCppProperty(Inner);
    Prop = ArrayProp;
    break;
  }
  case ERustealReifyPropType::SoftObject: {
    FSoftObjectProperty *SoftProp =
        new FSoftObjectProperty(Owner, PropName, RF_Public);
    SoftProp->PropertyClass =
        (Extra && Extra->class_handle.ptr)
            ? static_cast<UClass *>(Extra->class_handle.ptr)
            : UObject::StaticClass();
    Prop = SoftProp;
    break;
  }
  default:
    UE_LOG(LogRusteal, Error,
           TEXT("[Rusteal] CreatePropertyByType: unknown type %d"),
           static_cast<int>(PropType));
    return nullptr;
  }

  return Prop;
}

static void CopyParamsFromParentFunction(UFunction *NewFunc,
                                         UFunction *ParentFunc) {
  for (FField *SrcField = ParentFunc->ChildProperties; SrcField;
       SrcField = SrcField->Next) {
    FProperty *SrcProp = CastField<FProperty>(SrcField);

    if (!SrcProp) {
      continue;
    }

    ERustealReifyPropType PropType;
    FRustealReifyPropExtra Extra = {};

    if (SrcProp->IsA<FBoolProperty>()) {
      PropType = ERustealReifyPropType::Bool;
    } else if (SrcProp->IsA<FInt8Property>()) {
      PropType = ERustealReifyPropType::Int8;
    } else if (SrcProp->IsA<FInt16Property>()) {
      PropType = ERustealReifyPropType::Int16;
    } else if (SrcProp->IsA<FIntProperty>()) {
      PropType = ERustealReifyPropType::Int32;
    } else if (SrcProp->IsA<FInt64Property>()) {
      PropType = ERustealReifyPropType::Int64;
    } else if (SrcProp->IsA<FByteProperty>()) {
      PropType = ERustealReifyPropType::UInt8;
    } else if (SrcProp->IsA<FUInt16Property>()) {
      PropType = ERustealReifyPropType::UInt16;
    } else if (SrcProp->IsA<FUInt32Property>()) {
      PropType = ERustealReifyPropType::UInt32;
    } else if (SrcProp->IsA<FUInt64Property>()) {
      PropType = ERustealReifyPropType::UInt64;
    } else if (SrcProp->IsA<FFloatProperty>()) {
      PropType = ERustealReifyPropType::Float;
    } else if (SrcProp->IsA<FDoubleProperty>()) {
      PropType = ERustealReifyPropType::Double;
    } else if (SrcProp->IsA<FStrProperty>()) {
      PropType = ERustealReifyPropType::String;
    } else if (SrcProp->IsA<FNameProperty>()) {
      PropType = ERustealReifyPropType::Name;
    } else if (SrcProp->IsA<FTextProperty>()) {
      PropType = ERustealReifyPropType::Text;
    } else if (SrcProp->IsA<FStructProperty>()) {
      PropType = ERustealReifyPropType::Struct;
      Extra.struct_handle.ptr = CastField<FStructProperty>(SrcProp)->Struct;
    } else if (SrcProp->IsA<FClassProperty>()) {
      PropType = ERustealReifyPropType::Class;

      Extra.class_handle.ptr =
          CastField<FClassProperty>(SrcProp)->PropertyClass;

      Extra.meta_class_handle.ptr =
          CastField<FClassProperty>(SrcProp)->MetaClass;
    } else if (SrcProp->IsA<FObjectProperty>()) {
      PropType = ERustealReifyPropType::Object;

      Extra.class_handle.ptr =
          CastField<FObjectProperty>(SrcProp)->PropertyClass;
    } else if (SrcProp->IsA<FEnumProperty>()) {
      PropType = ERustealReifyPropType::Enum;
      Extra.enum_handle.ptr = CastField<FEnumProperty>(SrcProp)->GetEnum();
    } else {
      UE_LOG(LogRusteal, Warning,
             TEXT("[Rusteal] CopyParamsFromParent: unsupported property type "
                  "'%s' for param '%s', skipping"),
             *SrcProp->GetClass()->GetName(), *SrcProp->GetName());

      continue;
    }

    FProperty *NewProp = CreatePropertyByType(
        FFieldVariant(NewFunc), SrcProp->GetFName(), PropType, &Extra);

    if (NewProp) {
      NewProp->PropertyFlags = SrcProp->PropertyFlags;

      if (NewProp->HasAnyPropertyFlags(CPF_OutParm) &&
          !NewProp->HasAnyPropertyFlags(CPF_ReturnParm)) {
        NewFunc->FunctionFlags |= FUNC_HasOutParms;
      }

      if (!NewFunc->ChildProperties) {
        NewFunc->ChildProperties = NewProp;
      } else {
        FField *Last = NewFunc->ChildProperties;

        while (Last->Next) {
          Last = Last->Next;
        }

        Last->Next = NewProp;
      }
    }
  }
}

static FRustealClassReinstancer GClassReinstancer;

static TArray<TPair<UClass *, UClass *>> GReplacedClasses;

void RustealSetClassReinstancer(FRustealClassReinstancer Reinstancer) {
  GClassReinstancer = MoveTemp(Reinstancer);
}

static void RetireClass(URustealReifiedClass *Old) {
  Old->Library = nullptr;

  for (UField *Field = Old->Children; Field; Field = Field->Next) {
    if (URustealReifiedFunction *Function =
            Cast<URustealReifiedFunction>(Field)) {
      Function->Library = nullptr;
    }
  }

  const ERenameFlags Flags =
      REN_DontCreateRedirectors | REN_NonTransactional | REN_DoNotDirty;

#if WITH_EDITORONLY_DATA

  if (UBlueprint *Stub = Cast<UBlueprint>(Old->ClassGeneratedBy)) {
    Stub->RemoveFromRoot();
    Stub->ClearFlags(RF_Standalone | RF_Public);

    Stub->Rename(*MakeUniqueObjectName(GetTransientPackage(), Stub->GetClass(),
                                       *(TEXT("RUSTEAL_") + Stub->GetName()))
                      .ToString(),
                 GetTransientPackage(), Flags);
  }

  Old->ClassGeneratedBy = nullptr;
#endif

  Old->RemoveFromRoot();
  Old->ClearFlags(RF_Standalone | RF_Public);

  if (UObject *CDO = Old->GetDefaultObject(false)) {
    CDO->RemoveFromRoot();
    CDO->ClearFlags(RF_Standalone | RF_Public);
  }

  Old->Rename(*MakeUniqueObjectName(GetTransientPackage(), Old->GetClass(),
                                    *(TEXT("RUSTEAL_") + Old->GetName()))
                   .ToString(),
              GetTransientPackage(), Flags);

  Old->SetFlags(RF_Transient);

  Old->AddToRoot();
}

void RustealReifyReinstanceReplacedClasses() {
  if (GReplacedClasses.IsEmpty()) {
    return;
  }

  TArray<TPair<UClass *, UClass *>> Replaced = MoveTemp(GReplacedClasses);
  GReplacedClasses.Reset();

  UE_LOG(LogRusteal, Display,
         TEXT("[Rusteal] Hot reload: reinstancing %d changed class(es)"),
         Replaced.Num());

  if (GClassReinstancer) {
    GClassReinstancer(Replaced);
  }
}

static RustealUClassHandle CreateClassImpl(const uint8 *Name, uint32 NameLen,
                                           RustealUClassHandle Parent,
                                           uint64 RustTypeId, uint64 Shape) {
  UClass *ParentClass = static_cast<UClass *>(Parent.ptr);

  if (!ParentClass) {
    UE_LOG(LogRusteal, Error, TEXT("[Rusteal] CreateClass: null parent class"));
    return RustealUClassHandle{nullptr};
  }

  const FString ClassName = ReifyUtf8ToFString(Name, NameLen);

  UPackage *RustealPackage = RustealCurrentPackage();

  URustealReifiedClass *Existing =
      FindObject<URustealReifiedClass>(RustealPackage, *ClassName);

  const bool bChanged = Existing && (Existing->Shape != Shape ||
                                     Existing->GetSuperClass() != ParentClass);

  if (Existing && (!bChanged || !GClassReinstancer)) {
    Existing->RustTypeId = RustTypeId;
    Existing->Library = RustealCurrentLibrary();

    if (bChanged) {
      UE_LOG(LogRusteal, Warning,
             TEXT("[Rusteal] Hot reload: %s changed (its properties, functions "
                  "or parent), which only the editor can apply: restart to use "
                  "the new class."),
             *ClassName);
    } else {
      UE_LOG(LogRusteal, Display,
             TEXT("[Rusteal] Hot reload: reusing existing class %s (type_id: "
                  "%llu)"),
             *ClassName, RustTypeId);
    }

    return RustealUClassHandle{Existing};
  }

  if (Existing) {
    UE_LOG(LogRusteal, Display,
           TEXT("[Rusteal] Hot reload: %s changed, replacing it"), *ClassName);

    RetireClass(Existing);
  }

  URustealReifiedClass *NewClass = NewObject<URustealReifiedClass>(
      RustealPackage, FName(*ClassName), RF_Public | RF_Standalone);

  NewClass->RustTypeId = RustTypeId;
  NewClass->Library = RustealCurrentLibrary();
  NewClass->Shape = Shape;

  if (Existing) {
    GReplacedClasses.Emplace(Existing, NewClass);
  }

  UClass *NativeSuper = ParentClass;

  while (NativeSuper && !NativeSuper->HasAnyClassFlags(CLASS_Native)) {
    NativeSuper = NativeSuper->GetSuperClass();
  }

  NewClass->NativeSuperClass = NativeSuper ? NativeSuper : ParentClass;

  NewClass->SetSuperStruct(ParentClass);

  NewClass->ClassWithin = ParentClass->ClassWithin;
  NewClass->ClassConstructor = &URustealReifiedClass::RustealClassConstructor;

  constexpr EClassFlags ConfigRelatedFlags =
      EClassFlags(CLASS_Config | CLASS_DefaultConfig | CLASS_PerObjectConfig |
                  CLASS_ConfigDoNotCheckDefaults | CLASS_GlobalUserConfig |
                  CLASS_ProjectUserConfig | CLASS_PerPlatformConfig);

  NewClass->ClassFlags |=
      (ParentClass->ClassFlags & CLASS_Inherit & ~ConfigRelatedFlags) |
      CLASS_CompiledFromBlueprint;

#if WITH_EDITORONLY_DATA

  UBlueprint *StubBP =
      NewObject<UBlueprint>(RustealPackage, FName(*(TEXT("RS_") + ClassName)),
                            RF_Public | RF_Standalone);

  StubBP->GeneratedClass = NewClass;
  StubBP->SkeletonGeneratedClass = NewClass;
  StubBP->ParentClass = ParentClass;
  StubBP->BlueprintType = BPTYPE_Normal;
  StubBP->Status = BS_UpToDate;
  StubBP->AddToRoot();
  NewClass->ClassGeneratedBy = StubBP;

  NewClass->bCooked = true;
#endif

  NewClass->AddToRoot();

  UE_LOG(
      LogRusteal, Display,
      TEXT("[Rusteal] Created reified class: %s (parent: %s, type_id: %llu)"),
      *ClassName, *ParentClass->GetName(), RustTypeId);

  return RustealUClassHandle{NewClass};
}

static RustealFPropertyHandle
AddPropertyImpl(RustealUClassHandle Cls, const uint8 *Name, uint32 NameLen,
                uint32 PropType, uint64 PropFlags,
                const FRustealReifyPropExtra *Extra) {
  UStruct *Class = static_cast<UStruct *>(Cls.ptr);

  if (!Class) {
    return RustealFPropertyHandle{nullptr};
  }

  const FName PropName = ReifyUtf8ToFName(Name, NameLen);

  for (FProperty *P = Class->PropertyLink; P; P = P->PropertyLinkNext) {
    if (P->GetOwnerStruct() == Class && P->GetFName() == PropName) {
      UE_LOG(LogRusteal, Display,
             TEXT("[Rusteal] Hot reload: reusing existing property %s::%s"),
             *Class->GetName(), *PropName.ToString());

      return RustealFPropertyHandle{P};
    }
  }

  FProperty *Prop =
      CreatePropertyByType(FFieldVariant(Class), PropName,
                           static_cast<ERustealReifyPropType>(PropType), Extra);

  if (!Prop) {
    return RustealFPropertyHandle{nullptr};
  }

  Prop->PropertyFlags |= static_cast<EPropertyFlags>(PropFlags);

  if (FArrayProperty *ArrayProp = CastField<FArrayProperty>(Prop)) {
    ArrayProp->Inner->PropertyFlags |=
        static_cast<EPropertyFlags>(PropFlags) & CPF_PropagateToArrayInner;
  }

  Class->AddCppProperty(Prop);

  return RustealFPropertyHandle{Prop};
}

static RustealUFunctionHandle AddFunctionImpl(RustealUClassHandle Cls,
                                              const uint8 *Name, uint32 NameLen,
                                              uint64 CallbackId,
                                              uint32 FuncFlags) {
  UClass *Class = static_cast<UClass *>(Cls.ptr);

  if (!Class) {
    return RustealUFunctionHandle{nullptr};
  }

  const FString FuncName = ReifyUtf8ToFString(Name, NameLen);

  UFunction *ExistingFunc = Class->FindFunctionByName(
      FName(*FuncName), EIncludeSuperFlag::ExcludeSuper);

  if (ExistingFunc) {
    if (URustealReifiedFunction *Reified =
            Cast<URustealReifiedFunction>(ExistingFunc)) {
      Reified->CallbackId = CallbackId;
      Reified->Library = RustealCurrentLibrary();

      UE_LOG(
          LogRusteal, Display,
          TEXT(
              "[Rusteal] Hot reload: updated CallbackId for %s::%s (id: %llu)"),
          *Class->GetName(), *FuncName, CallbackId);

      return RustealUFunctionHandle{ExistingFunc};
    }
  }

  URustealReifiedFunction *NewFunc = NewObject<URustealReifiedFunction>(
      Class, FName(*FuncName), RF_Public | RF_MarkAsNative);

  const bool bScriptEvent =
      !(static_cast<EFunctionFlags>(FuncFlags) & FUNC_Native);

  NewFunc->CallbackId = CallbackId;
  NewFunc->Library = RustealCurrentLibrary();
  NewFunc->FunctionFlags = static_cast<EFunctionFlags>(FuncFlags);

  if (!bScriptEvent) {
    NewFunc->FunctionFlags |= FUNC_Native;

    NewFunc->SetNativeFunc(&URustealReifiedFunction::execCallRustFunction);
  }

  NewFunc->Next = Class->Children;
  Class->Children = NewFunc;

  if (!bScriptEvent &&
      (static_cast<EFunctionFlags>(FuncFlags) & FUNC_BlueprintEvent)) {
    UFunction *ParentFunc =
        Class->GetSuperClass()
            ? Class->GetSuperClass()->FindFunctionByName(NewFunc->GetFName())
            : nullptr;

    if (ParentFunc) {
      CopyParamsFromParentFunction(NewFunc, ParentFunc);

      UE_LOG(LogRusteal, Display,
             TEXT("[Rusteal] Override %s::%s: copied params from parent %s"),
             *Class->GetName(), *FuncName, *ParentFunc->GetOuter()->GetName());
    }
  }

  if (!bScriptEvent) {
    Class->AddNativeFunction(*FuncName,
                             &URustealReifiedFunction::execCallRustFunction);
  }

  Class->AddFunctionToFunctionMap(NewFunc, NewFunc->GetFName());

  return RustealUFunctionHandle{NewFunc};
}

static ERustealErrorCode
AddFunctionParamImpl(RustealUFunctionHandle Func, const uint8 *Name,
                     uint32 NameLen, uint32 PropType, uint64 ParamFlags,
                     const FRustealReifyPropExtra *Extra) {
  URustealReifiedFunction *Function =
      Cast<URustealReifiedFunction>(static_cast<UFunction *>(Func.ptr));

  if (!Function) {
    return ERustealErrorCode::NullArgument;
  }

  const FName ParamName = ReifyUtf8ToFName(Name, NameLen);

  for (FField *Field = Function->ChildProperties; Field; Field = Field->Next) {
    if (FProperty *Existing = CastField<FProperty>(Field)) {
      if (Existing->GetFName() == ParamName) {
        return ERustealErrorCode::Ok;
      }
    }
  }

  FProperty *Param =
      CreatePropertyByType(FFieldVariant(Function), ParamName,
                           static_cast<ERustealReifyPropType>(PropType), Extra);

  if (!Param) {
    return ERustealErrorCode::InternalError;
  }

  Param->PropertyFlags |= static_cast<EPropertyFlags>(ParamFlags) | CPF_Parm;

  if (Param->HasAnyPropertyFlags(CPF_OutParm) &&
      !Param->HasAnyPropertyFlags(CPF_ReturnParm)) {
    Function->FunctionFlags |= FUNC_HasOutParms;
  }

  if (Function->ChildProperties == nullptr) {
    Function->ChildProperties = Param;
  } else {
    FField *Last = Function->ChildProperties;

    while (Last->Next) {
      Last = Last->Next;
    }

    Last->Next = Param;
  }

  return ERustealErrorCode::Ok;
}

static ERustealErrorCode FinalizeClassImpl(RustealUClassHandle Cls) {
  URustealReifiedClass *Class =
      Cast<URustealReifiedClass>(static_cast<UClass *>(Cls.ptr));

  if (!Class) {
    return ERustealErrorCode::NullArgument;
  }

  Class->InvalidateCustomPropertyList();

  if (Class->HasAnyClassFlags(CLASS_Constructed)) {
    UE_LOG(LogRusteal, Display,
           TEXT("[Rusteal] Hot reload: class %s already finalized, skipping"),
           *Class->GetName());

    return ERustealErrorCode::Ok;
  }

  for (TFieldIterator<UFunction> FuncIt(Class,
                                        EFieldIteratorFlags::ExcludeSuper);
       FuncIt; ++FuncIt) {
    UFunction *Func = *FuncIt;
    Func->Bind();
    Func->StaticLink(true);
  }

  Class->Bind();
  Class->StaticLink(true);

  Class->AssembleReferenceTokenStream(true);

  NotifyRegistrationEvent(Class->GetOutermost()->GetFName(), Class->GetFName(),
                          ENotifyRegistrationType::NRT_Class,
                          ENotifyRegistrationPhase::NRP_Finished, nullptr,
                          false, Class);

  UObject *CDO = Class->GetDefaultObject(true);
  Class->PostLoadDefaultObject(CDO);

  if (AActor *ActorCDO = Cast<AActor>(CDO)) {
    for (TFieldIterator<UFunction> FuncIt(Class,
                                          EFieldIteratorFlags::ExcludeSuper);
         FuncIt; ++FuncIt) {
      if ((*FuncIt)->GetFName() == FName(TEXT("ReceiveTick"))) {
        ActorCDO->PrimaryActorTick.bCanEverTick = true;
        ActorCDO->PrimaryActorTick.bStartWithTickEnabled = true;

        UE_LOG(LogRusteal, Display,
               TEXT("[Rusteal] Enabled tick for %s (ReceiveTick override "
                    "detected)"),
               *Class->GetName());

        break;
      }
    }
  }

  UE_LOG(
      LogRusteal, Display,
      TEXT("[Rusteal] Finalized reified class: %s (size: %d, super_size: %d)"),
      *Class->GetName(), Class->GetPropertiesSize(),
      Class->GetSuperClass() ? Class->GetSuperClass()->GetPropertiesSize() : 0);

  int32 PropCount = 0;

  for (FProperty *P = Class->PropertyLink; P; P = P->PropertyLinkNext) {
    if (P->GetOwnerClass() == Class) {
      UE_LOG(LogRusteal, Display,
             TEXT("[Rusteal]   Property: %s offset=%d size=%d"), *P->GetName(),
             P->GetOffset_ForInternal(), P->GetSize());
    }

    PropCount++;

    if (PropCount > 10000) {
      UE_LOG(
          LogRusteal, Error,
          TEXT(
              "[Rusteal] PropertyLink chain appears corrupt (>10000 entries)"));

      break;
    }
  }

  UE_LOG(LogRusteal, Display, TEXT("[Rusteal]   Total properties in chain: %d"),
         PropCount);

  return ERustealErrorCode::Ok;
}

static RustealUObjectHandle GetCdoImpl(RustealUClassHandle Cls) {
  UClass *Class = static_cast<UClass *>(Cls.ptr);

  if (!Class) {
    return RustealUObjectHandle{nullptr};
  }

  return RustealUObjectHandle{Class->GetDefaultObject()};
}

static ERustealErrorCode AddDefaultSubobjectImpl(
    RustealUClassHandle Cls, const uint8 *Name, uint32 NameLen,
    const uint8 *Property, uint32 PropertyLen, RustealUClassHandle CompClass,
    uint32 Flags, const uint8 *AttachParent, uint32 AttachLen,
    const uint8 *AttachSocket, uint32 SocketLen) {
  URustealReifiedClass *RC =
      Cast<URustealReifiedClass>(static_cast<UClass *>(Cls.ptr));

  if (!RC)
    return ERustealErrorCode::InvalidCast;

  UClass *CompUClass = static_cast<UClass *>(CompClass.ptr);

  if (!CompUClass)
    return ERustealErrorCode::PropertyNotFound;

  FRustealComponentDef Def;
  Def.SubobjectName = FName(ReifyUtf8ToFString(Name, NameLen));
  Def.ComponentClass = CompUClass;
  Def.bIsRoot = (Flags & 1) != 0;
  Def.bIsTransient = (Flags & 2) != 0;

  Def.AttachParentName =
      AttachLen > 0 ? FName(ReifyUtf8ToFString(AttachParent, AttachLen))
                    : NAME_None;

  Def.AttachSocketName =
      SocketLen > 0 ? FName(ReifyUtf8ToFString(AttachSocket, SocketLen))
                    : NAME_None;

  RC->ComponentDefs.RemoveAll([&](const FRustealComponentDef &D) {
    return D.SubobjectName == Def.SubobjectName;
  });

  UE_LOG(LogRusteal, Display,
         TEXT("[Rusteal] Registered default subobject '%s' (class: %s) on %s"),
         *Def.SubobjectName.ToString(), *CompUClass->GetName(), *RC->GetName());

  Def.PropertyName = PropertyLen > 0 ? ReifyUtf8ToFName(Property, PropertyLen)
                                     : Def.SubobjectName;

  if (!FindFProperty<FProperty>(RC, Def.PropertyName)) {
    FObjectProperty *CompProp =
        new FObjectProperty(FFieldVariant(RC), Def.PropertyName);

    CompProp->PropertyClass = CompUClass;

    CompProp->PropertyFlags |= CPF_Edit | CPF_EditConst | CPF_BlueprintVisible |
                               CPF_BlueprintReadOnly | CPF_ExportObject |
                               CPF_InstancedReference | CPF_ZeroConstructor |
                               CPF_NoDestructor;

#if WITH_EDITORONLY_DATA
    CompProp->SetMetaData(TEXT("EditInline"), TEXT("true"));
#endif
    RC->AddCppProperty(CompProp);
  }

  RC->ComponentDefs.Add(MoveTemp(Def));

  return ERustealErrorCode::Ok;
}

static RustealUObjectHandle FindDefaultSubobjectImpl(RustealUObjectHandle Owner,
                                                     const uint8 *Name,
                                                     uint32 NameLen) {
  UObject *Obj = static_cast<UObject *>(Owner.ptr);

  if (!Obj)
    return RustealUObjectHandle{nullptr};

  FName SubName(ReifyUtf8ToFString(Name, NameLen));
  UObject *Sub = Obj->GetDefaultSubobjectByName(SubName);
  return RustealUObjectHandle{Sub};
}

static ERustealErrorCode
SetPropertyMetadataImpl(RustealFPropertyHandle Prop, const uint8 *Key,
                        uint32 KeyLen, const uint8 *Value, uint32 ValueLen) {
  FProperty *Property = static_cast<FProperty *>(Prop.ptr);

  if (!Property) {
    return ERustealErrorCode::NullArgument;
  }

#if WITH_EDITORONLY_DATA

  Property->SetMetaData(ReifyUtf8ToFName(Key, KeyLen),
                        ReifyUtf8ToFString(Value, ValueLen));

#endif
  return ERustealErrorCode::Ok;
}

class FRustealDeleteListener : public FUObjectArray::FUObjectDeleteListener {
public:
  virtual void NotifyUObjectDeleted(const UObjectBase *Object,
                                    int32 Index) override {
    const TArray<URustealReifiedClass *> Chain =
        URustealReifiedClass::ReifiedChain(Object->GetClass());

    TArray<FRustealLibrary *, TInlineAllocator<2>> Dropped;

    for (int32 ChainIndex = Chain.Num() - 1; ChainIndex >= 0; --ChainIndex) {
      const URustealReifiedClass *ReifiedClass = Chain[ChainIndex];
      FRustealLibrary *Library = ReifiedClass->Library;

      if (Dropped.Contains(Library)) {
        continue;
      }

      Dropped.Add(Library);

      RustealCallLibrary(
          Library, [Object, ReifiedClass](const FRustealRustCallbacks &Cb) {
            Cb.drop_rust_instance(
                RustealUObjectHandle{const_cast<UObjectBase *>(Object)},
                ReifiedClass->RustTypeId, nullptr);
          });
    }
  }

  virtual void OnUObjectArrayShutdown() override {
    GUObjectArray.RemoveUObjectDeleteListener(this);
  }
};

static FRustealDeleteListener GDeleteListener;

void RustealReifyRegisterDeleteListener() {
  GUObjectArray.AddUObjectDeleteListener(&GDeleteListener);
}

void RustealReifyUnregisterDeleteListener() {
  GUObjectArray.RemoveUObjectDeleteListener(&GDeleteListener);
}

void RustealReifyForEachReifiedInstance(
    TFunctionRef<void(UObject *, URustealReifiedClass *)> Callback) {
  TArray<UObject *> Objects;

  for (TObjectIterator<URustealReifiedClass> ClassIt; ClassIt; ++ClassIt) {
    URustealReifiedClass *ReifiedClass = *ClassIt;
    Objects.Reset();

    GetObjectsOfClass(ReifiedClass, Objects, true, RF_NoFlags);

    for (UObject *Obj : Objects) {
      Callback(Obj, ReifiedClass);
    }
  }
}

static TMap<TWeakObjectPtr<UScriptStruct>, uint64> GStructShapes;

static RustealUStructHandle CreateStructImpl(const uint8 *Name, uint32 NameLen,
                                             uint64 Shape) {
  UPackage *RustealPackage = RustealCurrentPackage();
  const FName StructName = ReifyUtf8ToFName(Name, NameLen);

  if (UScriptStruct *Existing =
          FindObject<UScriptStruct>(RustealPackage, *StructName.ToString())) {
    const uint64 *OldShape = GStructShapes.Find(Existing);

    if (OldShape && *OldShape != Shape) {
      UE_LOG(
          LogRusteal, Warning,
          TEXT("[Rusteal] Hot reload: the fields of struct %s changed, which "
               "a hot reload does not apply: restart the editor to use them."),
          *StructName.ToString());
    }

    return RustealUStructHandle{Existing};
  }

  UScriptStruct *NewStruct = NewObject<UScriptStruct>(
      RustealPackage, StructName, RF_Public | RF_Standalone);

  GStructShapes.Add(NewStruct, Shape);
#if WITH_EDITORONLY_DATA

  NewStruct->SetMetaData(TEXT("BlueprintType"), TEXT("true"));
#endif
  return RustealUStructHandle{NewStruct};
}

static ERustealErrorCode FinalizeStructImpl(RustealUStructHandle Handle) {
  UScriptStruct *Struct = static_cast<UScriptStruct *>(Handle.ptr);

  if (!Struct) {
    return ERustealErrorCode::NullArgument;
  }

  if (Struct->GetStructureSize() > 0) {
    return ERustealErrorCode::Ok;
  }

  for (TFieldIterator<FStructProperty> It(Struct,
                                          EFieldIteratorFlags::ExcludeSuper);
       It; ++It) {
    if (It->Struct && It->Struct->GetStructureSize() == 0) {
      FinalizeStructImpl(RustealUStructHandle{It->Struct});
    }
  }

  Struct->Bind();
  Struct->StaticLink(true);
  Struct->PrepareCppStructOps();

  NotifyRegistrationEvent(
      Struct->GetOutermost()->GetFName(), Struct->GetFName(),
      ENotifyRegistrationType::NRT_Struct,
      ENotifyRegistrationPhase::NRP_Finished, nullptr, false, Struct);

  UE_LOG(LogRusteal, Display, TEXT("[Rusteal] Finalized struct: %s (size: %d)"),
         *Struct->GetName(), Struct->GetStructureSize());

  return ERustealErrorCode::Ok;
}

static RustealUFunctionHandle AddDelegateImpl(RustealUClassHandle Cls,
                                              const uint8 *Name, uint32 NameLen,
                                              uint64 PropFlags) {
  UClass *Class = static_cast<UClass *>(Cls.ptr);

  if (!Class) {
    return RustealUFunctionHandle{nullptr};
  }

  const FName PropName = ReifyUtf8ToFName(Name, NameLen);

  for (TFieldIterator<FMulticastDelegateProperty> It(
           Class, EFieldIteratorFlags::ExcludeSuper);
       It; ++It) {
    if (It->GetFName() == PropName) {
      return RustealUFunctionHandle{It->SignatureFunction};
    }
  }

  const FName SignatureName(
      *FString::Printf(TEXT("%s__DelegateSignature"), *PropName.ToString()));

  URustealReifiedFunction *Signature =
      NewObject<URustealReifiedFunction>(Class, SignatureName, RF_Public);

  Signature->FunctionFlags =
      FUNC_Public | FUNC_Delegate | FUNC_MulticastDelegate;

  Signature->CallbackId = 0;

  Signature->Next = Class->Children;
  Class->Children = Signature;

  FMulticastInlineDelegateProperty *Prop = new FMulticastInlineDelegateProperty(
      FFieldVariant(Class), PropName, RF_Public);

  Prop->SignatureFunction = Signature;

  Prop->PropertyFlags |= CPF_BlueprintAssignable | CPF_BlueprintCallable |
                         static_cast<EPropertyFlags>(PropFlags);

  Class->AddCppProperty(Prop);

  return RustealUFunctionHandle{Signature};
}

static TArray<TPair<TWeakObjectPtr<UClass>, FString>> GPendingInterfaces;
static bool GInterfacesResolvable = false;

static void ResolveInterface(UClass *Class, const FString &Path) {
  UClass *Interface = LoadClass<UInterface>(nullptr, *Path);

  if (!Interface || !Interface->HasAnyClassFlags(CLASS_Interface)) {
    UE_LOG(LogRusteal, Error,
           TEXT("[Rusteal] %s implements %s: no interface at that path"),
           *Class->GetName(), *Path);

    return;
  }

#if WITH_METADATA

  if (Interface->HasMetaData(TEXT("CannotImplementInterfaceInBlueprint"))) {
    UE_LOG(LogRusteal, Error,
           TEXT("[Rusteal] %s implements %s: a C++-only interface (its "
                "functions are not Blueprint events), which only C++ can "
                "implement"),
           *Class->GetName(), *Path);

    return;
  }

#endif

  if (Class->ImplementsInterface(Interface)) {
    return;
  }

  Class->Interfaces.Add(FImplementedInterface(Interface, 0, true));

  UE_LOG(LogRusteal, Display, TEXT("[Rusteal] %s implements %s"),
         *Class->GetName(), *Interface->GetPathName());
}

static void ResolvePendingInterfaces() {
  GInterfacesResolvable = true;

  for (const TPair<TWeakObjectPtr<UClass>, FString> &Pending :
       GPendingInterfaces) {
    if (UClass *Class = Pending.Key.Get()) {
      ResolveInterface(Class, Pending.Value);
    }
  }

  GPendingInterfaces.Empty();
}

static ERustealErrorCode AddInterfaceImpl(RustealUClassHandle Cls,
                                          const uint8 *Path, uint32 PathLen) {
  UClass *Class = static_cast<UClass *>(Cls.ptr);

  if (!Class || !Path) {
    return ERustealErrorCode::NullArgument;
  }

  const FString InterfacePath = ReifyUtf8ToFString(Path, PathLen);

  if (GInterfacesResolvable || (GEngine && GEngine->IsInitialized())) {
    GInterfacesResolvable = true;
    ResolveInterface(Class, InterfacePath);
    return ERustealErrorCode::Ok;
  }

  if (GPendingInterfaces.IsEmpty()) {
    FCoreDelegates::OnPostEngineInit.AddStatic(&ResolvePendingInterfaces);
  }

  GPendingInterfaces.Emplace(Class, InterfacePath);
  return ERustealErrorCode::Ok;
}

static ERustealErrorCode SetClassConfigImpl(RustealUClassHandle Cls,
                                            const uint8 *Name, uint32 NameLen) {
  URustealReifiedClass *Class =
      Cast<URustealReifiedClass>(static_cast<UClass *>(Cls.ptr));

  if (!Class || !Name || NameLen == 0) {
    return ERustealErrorCode::NullArgument;
  }

  Class->ClassConfigName = ReifyUtf8ToFName(Name, NameLen);
  Class->ClassFlags |= CLASS_Config | CLASS_DefaultConfig;

  if (UObject *CDO = Class->GetDefaultObject(false)) {
    CDO->LoadConfig();
  }

  return ERustealErrorCode::Ok;
}

FRustealReifyApi GReifyApi = {
    &CreateClassImpl,         &AddPropertyImpl,
    &AddFunctionImpl,         &AddFunctionParamImpl,
    &FinalizeClassImpl,       &GetCdoImpl,
    &AddDefaultSubobjectImpl, &FindDefaultSubobjectImpl,
    &SetPropertyMetadataImpl, &CreateStructImpl,
    &FinalizeStructImpl,      &AddDelegateImpl,
    &AddInterfaceImpl,        &SetClassConfigImpl,
};
