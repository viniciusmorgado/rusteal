#include "URustealReifiedClass.h"
#include "Components/SceneComponent.h"
#include "Engine/Blueprint.h"
#include "GameFramework/Actor.h"
#include "RustealApiTable.h"
#include "RustealLibrary.h"
#include "RustealModule.h"

UClass *URustealReifiedClass::GetAuthoritativeClass() { return this; }

void URustealReifiedClass::InitPropertiesFromCustomList(
    uint8 *DataPtr, const uint8 *DefaultDataPtr) {
  if (!bCustomPropertyListCurrent.load()) {
    FScopeLock Lock(&CustomPropertyListLock);

    if (!bCustomPropertyListCurrent.load()) {
      UpdateCustomPropertyListForPostConstruction();
      bCustomPropertyListCurrent.store(true);
    }
  }

  Super::InitPropertiesFromCustomList(DataPtr, DefaultDataPtr);
}

void URustealReifiedClass::InvalidateCustomPropertyList() {
  bCustomPropertyListCurrent.store(false);
}

static USceneComponent *FindInheritedComponent(UObject *Obj, FName Name) {
  if (const FObjectPropertyBase *Prop =
          FindFProperty<FObjectPropertyBase>(Obj->GetClass(), Name)) {
    if (USceneComponent *Component = Cast<USceneComponent>(
            Prop->GetObjectPropertyValue_InContainer(Obj))) {
      return Component;
    }
  }

  return Cast<USceneComponent>(Obj->GetDefaultSubobjectByName(Name));
}

void URustealReifiedClass::RustealClassConstructor(
    const FObjectInitializer &ObjectInitializer) {
  const TArray<URustealReifiedClass *> Chain =
      ReifiedChain(ObjectInitializer.GetClass());

  URustealReifiedClass *ReifiedClass = Chain.Num() > 0 ? Chain.Last() : nullptr;

  if (!ReifiedClass) {
    UE_LOG(
        LogRusteal, Error,
        TEXT("[Rusteal] RustealClassConstructor called on non-reified class!"));

    return;
  }

  UClass *NativeSuper = ReifiedClass->NativeSuperClass;

  if (NativeSuper && NativeSuper->ClassConstructor) {
    NativeSuper->ClassConstructor(ObjectInitializer);
  }

  UObject *Obj = ObjectInitializer.GetObj();
  TMap<FName, USceneComponent *> CreatedComponents;

  for (URustealReifiedClass *DefClass : Chain) {
    for (const FRustealComponentDef &Def : DefClass->ComponentDefs) {
      UObject *Sub = ObjectInitializer.CreateDefaultSubobject(
          Obj, Def.SubobjectName, Def.ComponentClass, Def.ComponentClass, true,
          Def.bIsTransient);

      if (!Sub)
        continue;

      if (FObjectProperty *CompProp =
              FindFProperty<FObjectProperty>(DefClass, Def.PropertyName)) {
        CompProp->SetObjectPropertyValue_InContainer(Obj, Sub);
      }

      USceneComponent *SceneComp = Cast<USceneComponent>(Sub);

      if (SceneComp) {
        CreatedComponents.Add(Def.PropertyName, SceneComp);
      }

      if (Def.bIsRoot) {
        if (AActor *Actor = Cast<AActor>(Obj)) {
          if (SceneComp)
            Actor->SetRootComponent(SceneComp);
        }
      } else if (Def.AttachParentName != NAME_None && SceneComp) {
        USceneComponent **Own = CreatedComponents.Find(Def.AttachParentName);

        USceneComponent *Parent =
            Own ? *Own : FindInheritedComponent(Obj, Def.AttachParentName);

        if (Parent) {
          SceneComp->SetupAttachment(Parent, Def.AttachSocketName);
        } else if (Obj->HasAnyFlags(RF_ClassDefaultObject)) {
          UE_LOG(LogRusteal, Warning,
                 TEXT("[Rusteal] %s: no component '%s' to attach '%s' to"),
                 *DefClass->GetName(), *Def.AttachParentName.ToString(),
                 *Def.SubobjectName.ToString());
        }
      }
    }
  }

  const bool bIsCDO = Obj->HasAnyFlags(RF_ClassDefaultObject);

  for (URustealReifiedClass *DataClass : Chain) {
    RustealCallLibrary(
        DataClass->Library,
        [Obj, DataClass, bIsCDO](const FRustealRustCallbacks &Cb) {
          Cb.construct_rust_instance(RustealUObjectHandle{Obj},
                                     DataClass->RustTypeId, bIsCDO);
        });
  }
}

TArray<URustealReifiedClass *>
URustealReifiedClass::ReifiedChain(const UClass *Class) {
  TArray<URustealReifiedClass *> Chain;

  for (const UClass *Cls = Class; Cls; Cls = Cls->GetSuperClass()) {
    if (const URustealReifiedClass *Reified = Cast<URustealReifiedClass>(Cls)) {
      Chain.Insert(const_cast<URustealReifiedClass *>(Reified), 0);
    }
  }

  return Chain;
}

#if WITH_EDITOR
static void ResyncOwnedComponents(UObject *Object) {
  AActor *Defaults = Cast<AActor>(Object);

  if (!Defaults || !Defaults->HasAnyFlags(RF_ClassDefaultObject)) {
    return;
  }

  for (UClass *Class = Defaults->GetClass(); Class;
       Class = Class->GetSuperClass()) {
    if (const URustealReifiedClass *Reified =
            Cast<URustealReifiedClass>(Class)) {
      if (Reified->ComponentDefs.Num() > 0) {
        Defaults->ResetOwnedComponents();
      }

      return;
    }
  }
}

static FDelegateHandle GPostCDOCompiledHandle;
static FDelegateHandle GAssetLoadedHandle;

#endif

void RustealRegisterComponentListResync() {
#if WITH_EDITOR

  GPostCDOCompiledHandle =
      FCoreUObjectDelegates::OnObjectPostCDOCompiled.AddLambda(
          [](UObject *Defaults, const FObjectPostCDOCompiledContext &) {
            ResyncOwnedComponents(Defaults);
          });

  GAssetLoadedHandle =
      FCoreUObjectDelegates::OnAssetLoaded.AddLambda([](UObject *Asset) {
        if (const UBlueprint *Blueprint = Cast<UBlueprint>(Asset)) {
          if (Blueprint->GeneratedClass) {
            ResyncOwnedComponents(
                Blueprint->GeneratedClass->GetDefaultObject(false));
          }
        }
      });

#endif
}

void RustealUnregisterComponentListResync() {
#if WITH_EDITOR
  FCoreUObjectDelegates::OnObjectPostCDOCompiled.Remove(GPostCDOCompiledHandle);
  FCoreUObjectDelegates::OnAssetLoaded.Remove(GAssetLoadedHandle);
#endif
}
