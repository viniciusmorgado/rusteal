#include "URustealReifiedClass.h"
#include "Components/SceneComponent.h"
#include "Engine/Blueprint.h"
#include "GameFramework/Actor.h"
#include "RustealApiTable.h"
#include "RustealLibrary.h"
#include "RustealModule.h"

UClass *URustealReifiedClass::GetAuthoritativeClass() {
  // Reified classes have no UBlueprint asset, so this class IS the
  // authoritative class.  The base implementation would crash on the
  // null ClassGeneratedBy pointer.
  return this;
}

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

// An inherited scene component of Obj by name: the component a property of
// that name points to (RootComponent, Mesh), or a default subobject of that
// name (CollisionCylinder).
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
  // 1. Find the Rust classes in the hierarchy. The immediate class may be a
  //    Blueprint child (e.g. SKEL_new_MacroTestActor_C), and a Rust class
  //    may have Rust parents: each adds its components and its Rust data.
  const TArray<URustealReifiedClass *> Chain =
      ReifiedChain(ObjectInitializer.GetClass());
  URustealReifiedClass *ReifiedClass = Chain.Num() > 0 ? Chain.Last() : nullptr;
  if (!ReifiedClass) {
    UE_LOG(
        LogRusteal, Error,
        TEXT("[Rusteal] RustealClassConstructor called on non-reified class!"));
    return;
  }

  // 2. Call the native super's constructor to initialize UE-side state.
  UClass *NativeSuper = ReifiedClass->NativeSuperClass;
  if (NativeSuper && NativeSuper->ClassConstructor) {
    NativeSuper->ClassConstructor(ObjectInitializer);
  }

  // 3. Create default subobjects from Rust-registered definitions, a Rust
  //    parent's first.
  UObject *Obj = ObjectInitializer.GetObj();
  TMap<FName, USceneComponent *> CreatedComponents;
  for (URustealReifiedClass *DefClass : Chain) {
    for (const FRustealComponentDef &Def : DefClass->ComponentDefs) {
      UObject *Sub = ObjectInitializer.CreateDefaultSubobject(
          Obj, Def.SubobjectName, Def.ComponentClass, Def.ComponentClass,
          /*bIsRequired=*/true, Def.bIsTransient);

      if (!Sub)
        continue;

      // Point the component's property at it (see AddDefaultSubobjectImpl).
      if (FObjectProperty *CompProp =
              FindFProperty<FObjectProperty>(DefClass, Def.PropertyName)) {
        CompProp->SetObjectPropertyValue_InContainer(Obj, Sub);
      }

      // Later components attach to it by its property name (the Rust field).
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
        // A component this class declared earlier, or an inherited one.
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

  // 4. Notify Rust to construct its instance data, one per Rust class, each
  // in the library that defines the class.
  const bool bIsCDO = Obj->HasAnyFlags(RF_ClassDefaultObject);
  for (URustealReifiedClass *DataClass : Chain) {
    RustealCallLibrary(DataClass->Library, [Obj, DataClass, bIsCDO](
                                               const FRustealRustCallbacks &Cb) {
      Cb.construct_rust_instance(RustealUObjectHandle{Obj},
                                 DataClass->RustTypeId, bIsCDO);
    });
  }
}

TArray<URustealReifiedClass *>
URustealReifiedClass::ReifiedChain(const UClass *Class) {
  TArray<URustealReifiedClass *> Chain;
  for (const UClass *Cls = Class; Cls; Cls = Cls->GetSuperClass()) {
    if (const URustealReifiedClass *Reified =
            Cast<URustealReifiedClass>(Cls)) {
      Chain.Insert(const_cast<URustealReifiedClass *>(Reified), 0);
    }
  }
  return Chain;
}

// ---------------------------------------------------------------------------
// Component list of Blueprint children's default objects
// ---------------------------------------------------------------------------

// A Blueprint child's default object ends up without the Rust components in
// its component list (AActor::OwnedComponents) once the editor has loaded or
// compiled it, although they are still its subobjects, owned by it; the
// constructor lists them. The Blueprint editor's component tree reads that
// list, so the components were missing there. Instances are not affected:
// AActor::PostInitProperties rebuilds the list. Rebuild it on the default
// object once it is final.
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

void RustealRegisterComponentListResync() {
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
}

void RustealUnregisterComponentListResync() {
  FCoreUObjectDelegates::OnObjectPostCDOCompiled.Remove(GPostCDOCompiledHandle);
  FCoreUObjectDelegates::OnAssetLoaded.Remove(GAssetLoadedHandle);
}
