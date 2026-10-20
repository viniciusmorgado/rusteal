#include "RustealLatentProxy.h"
#include "Engine/Engine.h"
#include "Engine/World.h"
#include "RustealApiTable.h"
#include "RustealErrors.h"
#include "RustealLibrary.h"
#include "UObject/GCObject.h"
#include "UObject/Package.h"

namespace {
class FRustealLatentProxies : public FGCObject {
public:
  TMap<uint64, TObjectPtr<URustealLatentProxy>> Proxies;

  virtual void AddReferencedObjects(FReferenceCollector &Collector) override {
    Collector.AddReferencedObjects(Proxies);
  }

  virtual FString GetReferencerName() const override {
    return TEXT("FRustealLatentProxies");
  }
};

FRustealLatentProxies &LatentProxies() {
  static FRustealLatentProxies *Proxies = new FRustealLatentProxies();
  return *Proxies;
}
} // namespace

URustealLatentProxy *URustealLatentProxy::Create(UObject *Context,
                                                 uint64 CallbackId) {
  if (!IsValid(Context)) {
    return nullptr;
  }

  URustealLatentProxy *Proxy = NewObject<URustealLatentProxy>(
      GetTransientPackage(), NAME_None, RF_Transient);

  Proxy->CallbackId = CallbackId;
  Proxy->Library = RustealCurrentLibrary();
  Proxy->Context = Context;
  LatentProxies().Proxies.Add(CallbackId, Proxy);
  return Proxy;
}

FLatentActionInfo URustealLatentProxy::MakeLatentInfo() {
  static int32 NextUuid = 1;

  return FLatentActionInfo(0, NextUuid++, TEXT("LatentDone"), this);
}

void URustealLatentProxy::LatentDone(int32 Linkage) {
  if (bFinished) {
    return;
  }

  bFinished = true;

  if (OnComplete) {
    OnComplete();
    OnComplete = nullptr;
  }

  RustealCallLibrary(Library, [this](const FRustealRustCallbacks &Cb) {
    Cb.invoke_delegate_callback(CallbackId, nullptr);
  });

  LatentProxies().Proxies.Remove(CallbackId);
}

void URustealLatentProxy::Cancel() {
  bFinished = true;
  OnComplete = nullptr;

  if (GEngine) {
    for (const FWorldContext &WorldContext : GEngine->GetWorldContexts()) {
      if (UWorld *World = WorldContext.World()) {
        World->GetLatentActionManager().RemoveActionsForObject(this);
      }
    }
  }

  LatentProxies().Proxies.Remove(CallbackId);
}

void URustealLatentProxy::BeginDestroy() {
  OnComplete = nullptr;

  if (CallbackId != 0) {
    RustealCallLibrary(Library, [this](const FRustealRustCallbacks &Cb) {
      Cb.release_delegate_callback(CallbackId);
    });
  }

  Super::BeginDestroy();
}

UWorld *URustealLatentProxy::GetWorld() const {
  if (UObject *Owner = Context.Get()) {
    return Owner->GetWorld();
  }

  return nullptr;
}

ERustealErrorCode RustealLatent_Cancel(uint64 CallbackId) {
  TObjectPtr<URustealLatentProxy> *Found =
      LatentProxies().Proxies.Find(CallbackId);

  if (Found && *Found) {
    (*Found)->Cancel();
  }

  return ERustealErrorCode::Ok;
}

uint32 RustealLatent_LiveProxies() {
  return static_cast<uint32>(LatentProxies().Proxies.Num());
}

void RustealLatentProxiesForgetLibrary(FRustealLibrary *Library) {
  TArray<TObjectPtr<URustealLatentProxy>> Owned;

  for (const TPair<uint64, TObjectPtr<URustealLatentProxy>> &Entry :
       LatentProxies().Proxies) {
    if (Entry.Value && Entry.Value->Library == Library) {
      Owned.Add(Entry.Value);
    }
  }

  for (URustealLatentProxy *Proxy : Owned) {
    Proxy->Library = nullptr;
    Proxy->Cancel();
    Proxy->CallbackId = 0;
  }
}
