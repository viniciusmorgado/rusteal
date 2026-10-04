#include "Kismet2/ReloadUtilities.h"
#include "Modules/ModuleManager.h"
#include "RustealAutoReload.h"
#include "RustealLibraries.h"

// The objects of the Rust classes a hot reload replaced move to the new
// classes through the engine's own reload, as Live Coding does with
// reinstancing on: FReload takes each (new, old) pair and reinstances the
// objects, the default object and the Blueprint children of the old class
// (docs/hot-reload.md).
static void ReinstanceClasses(const TArray<TPair<UClass *, UClass *>> &Classes) {
  FReload Reload(EActiveReloadType::Reinstancing, TEXT("RUSTEAL"), *GLog);
  for (const TPair<UClass *, UClass *> &Pair : Classes) {
    Reload.NotifyChange(Pair.Value, Pair.Key);
  }
  Reload.Reinstance();
  Reload.Finalize(/*bRunGC=*/true);
}

class FRustealEditorModule : public IModuleInterface {
public:
  virtual void StartupModule() override {
    RustealSetClassReinstancer(&ReinstanceClasses);
    AutoReload.Start();
  }

  virtual void ShutdownModule() override {
    AutoReload.Stop();
    RustealSetClassReinstancer(nullptr);
  }

private:
  FRustealAutoReload AutoReload;
};

IMPLEMENT_MODULE(FRustealEditorModule, RustealEditor)
