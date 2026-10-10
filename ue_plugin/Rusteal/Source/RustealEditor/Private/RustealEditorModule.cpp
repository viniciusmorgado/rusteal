#include "BlueprintActionDatabase.h"
#include "Engine/Blueprint.h"
#include "K2Node.h"
#include "Kismet2/BlueprintEditorUtils.h"
#include "Kismet2/KismetEditorUtilities.h"
#include "Kismet2/ReloadUtilities.h"
#include "Modules/ModuleManager.h"
#include "UObject/UObjectIterator.h"

DEFINE_LOG_CATEGORY_STATIC(LogRustealReload, Log, All);
#include "RustealAutoReload.h"
#include "RustealLibraries.h"
#include "URustealReifiedClass.h"

static bool UsesAny(UBlueprint *Blueprint, const TArray<UClass *> &Classes) {
  TArray<UK2Node *> Nodes;
  FBlueprintEditorUtils::GetAllNodesOfClass(Blueprint, Nodes);
  TArray<UStruct *> Dependencies;

  for (UK2Node *Node : Nodes) {
    Dependencies.Reset();

    if (!Node->HasExternalDependencies(&Dependencies)) {
      continue;
    }

    for (UStruct *Dependency : Dependencies) {
      const UClass *Class = Cast<UClass>(Dependency);

      for (UClass *Replaced : Classes) {
        if (Class && Class->IsChildOf(Replaced)) {
          return true;
        }
      }
    }
  }

  return false;
}

static void
CompileBlueprintsUsing(const TArray<TPair<UClass *, UClass *>> &Replaced) {
  TArray<UClass *> Classes;

  for (const TPair<UClass *, UClass *> &Pair : Replaced) {
    Classes.Add(Pair.Key);
    Classes.Add(Pair.Value);
  }

  TArray<UBlueprint *> Users;

  for (TObjectIterator<UBlueprint> It; It; ++It) {
    UBlueprint *Blueprint = *It;
    UClass *Generated = Blueprint->GeneratedClass;

    if (!Generated ||
        Blueprint->HasAnyFlags(RF_NeedLoad | RF_ClassDefaultObject) ||
        Blueprint->GetOutermost() == GetTransientPackage() ||
        Generated->HasAnyClassFlags(CLASS_NewerVersionExists)) {
      continue;
    }

    const bool bChild = Classes.ContainsByPredicate(
        [Generated](UClass *Class) { return Generated->IsChildOf(Class); });

    if (!bChild && UsesAny(Blueprint, Classes)) {
      Users.Add(Blueprint);
    }
  }

  for (UBlueprint *Blueprint : Users) {
    UE_LOG(LogRustealReload, Display,
           TEXT("Recompiling %s, which uses a replaced class"),
           *Blueprint->GetPathName());

    FKismetEditorUtilities::CompileBlueprint(
        Blueprint, EBlueprintCompileOptions::SkipGarbageCollection);
  }
}

static void
ReinstanceClasses(const TArray<TPair<UClass *, UClass *>> &Classes) {
  FReload Reload(EActiveReloadType::Reinstancing, TEXT("RUSTEAL"), *GLog);

  for (const TPair<UClass *, UClass *> &Pair : Classes) {
    Reload.NotifyChange(Pair.Value, Pair.Key);
  }

  Reload.Reinstance();

  CompileBlueprintsUsing(Classes);

  Reload.Finalize(false);
}

static void RefreshInterfaces(const TArray<UClass *> &Interfaces) {
  TArray<UBlueprint *> Users;

  for (TObjectIterator<UBlueprint> It; It; ++It) {
    UBlueprint *Blueprint = *It;
    UClass *Generated = Blueprint->GeneratedClass;

    if (!Generated ||
        Blueprint->HasAnyFlags(RF_NeedLoad | RF_ClassDefaultObject) ||
        Blueprint->GetOutermost() == GetTransientPackage() ||
        Generated->HasAnyClassFlags(CLASS_NewerVersionExists) ||
        (Cast<URustealReifiedClass>(Generated) &&
         Generated->ClassGeneratedBy == Blueprint)) {
      continue;
    }

    const bool bImplements =
        Interfaces.ContainsByPredicate([Generated](UClass *Interface) {
          return Generated->ImplementsInterface(Interface);
        });

    if (bImplements || UsesAny(Blueprint, Interfaces)) {
      Users.Add(Blueprint);
    }
  }

  for (UBlueprint *Blueprint : Users) {
    UE_LOG(LogRustealReload, Display,
           TEXT("Recompiling %s, which uses a changed interface"),
           *Blueprint->GetPathName());

    FBlueprintEditorUtils::ConformImplementedInterfaces(Blueprint);
    FBlueprintEditorUtils::RefreshAllNodes(Blueprint);

    FKismetEditorUtilities::CompileBlueprint(
        Blueprint, EBlueprintCompileOptions::SkipGarbageCollection);
  }

  if (FBlueprintActionDatabase *Actions = FBlueprintActionDatabase::TryGet()) {
    for (UClass *Interface : Interfaces) {
      Actions->RefreshClassActions(Interface);
    }
  }
}

class FRustealEditorModule : public IModuleInterface {
public:
  virtual void StartupModule() override {
    RustealSetClassReinstancer(&ReinstanceClasses);
    RustealSetInterfaceRefresher(&RefreshInterfaces);
    AutoReload.Start();
  }

  virtual void ShutdownModule() override {
    AutoReload.Stop();
    RustealSetInterfaceRefresher(nullptr);
    RustealSetClassReinstancer(nullptr);
  }

private:
  FRustealAutoReload AutoReload;
};

IMPLEMENT_MODULE(FRustealEditorModule, RustealEditor)
