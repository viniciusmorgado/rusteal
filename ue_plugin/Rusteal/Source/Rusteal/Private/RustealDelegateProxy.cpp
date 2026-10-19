#include "RustealDelegateProxy.h"
#include "RustealApiTable.h"
#include "RustealLibrary.h"
#include "Engine/World.h"
#include "RustealModule.h"
#include "UObject/GCObject.h"
#include "UObject/UObjectIterator.h"

FName URustealDelegateProxy::FakeFuncName(TEXT("RustFakeCallable"));

namespace {
class FRustealDelegateProxies : public FGCObject {
public:
  TArray<TObjectPtr<URustealDelegateProxy>> Proxies;

  virtual void AddReferencedObjects(FReferenceCollector &Collector) override {
    Collector.AddReferencedObjects(Proxies);
  }

  virtual FString GetReferencerName() const override {
    return TEXT("FRustealDelegateProxies");
  }
};

FRustealDelegateProxies &DelegateProxies() {
  static FRustealDelegateProxies *Proxies = new FRustealDelegateProxies();
  return *Proxies;
}
} // namespace

void URustealDelegateProxy::Keep(URustealDelegateProxy *Proxy) {
  TArray<TObjectPtr<URustealDelegateProxy>> &Proxies = DelegateProxies().Proxies;

  Proxies.RemoveAll([](const TObjectPtr<URustealDelegateProxy> &Kept) {
    return !Kept || !Kept->OwnerObject.IsValid();
  });

  Proxies.Add(Proxy);
}

void URustealDelegateProxy::Release(URustealDelegateProxy *Proxy) {
  DelegateProxies().Proxies.Remove(Proxy);
}

void RustealDelegateProxiesForgetLibrary(FRustealLibrary *Library) {
  DelegateProxies().Proxies.RemoveAll(
      [Library](const TObjectPtr<URustealDelegateProxy> &Kept) {
        return !Kept || Kept->Library == Library;
      });

  for (TObjectIterator<URustealDelegateProxy> It; It; ++It) {
    if (It->Library == Library) {
      It->Library = nullptr;
      It->CallbackId = 0;
    }
  }
}

void URustealDelegateProxy::RustFakeCallable() {}

UWorld *URustealDelegateProxy::GetWorld() const {
  UObject *Owner = OwnerObject.Get();

  if (Owner && Owner != GetTransientPackage()) {
    return Owner->GetWorld();
  }

  return GWorld;
}

void URustealDelegateProxy::BeginDestroy() {
  if (Library && Library->IsLoaded() && CallbackId != 0) {
    const uint64 Id = CallbackId;

    RustealCallLibrary(Library, [Id](const FRustealRustCallbacks &Cb) {
      Cb.release_delegate_callback(Id);
    });
  }

  CallbackId = 0;
  Super::BeginDestroy();
}

void URustealDelegateProxy::ProcessEvent(UFunction *Function, void *Parms) {
  if (Function->GetFName() != FakeFuncName) {
    Super::ProcessEvent(Function, Parms);
    return;
  }

  if (Library && Library->IsLoaded()) {
    RustealCallLibrary(Library, [this, Parms](const FRustealRustCallbacks &Cb) {
      Cb.invoke_delegate_callback(CallbackId, static_cast<uint8 *>(Parms));
    });
  } else {
    UE_LOG(LogRusteal, Warning,
           TEXT("[Rusteal] DelegateProxy: Rust callbacks not available "
                "(CallbackId=%llu)"),
           CallbackId);
  }
}
