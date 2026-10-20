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

namespace {
class FRustealLatentAction;

TMap<uint64, FRustealLatentAction *> GLatentActions;
uint64 GNextLatentAction = 1;

class FRustealLatentAction : public FPendingLatentAction {
public:
  FName ExecutionFunction;
  int32 OutputLink;
  FWeakObjectPtr CallbackTarget;
  FRustealLibrary *Library;
  uint64 AbortCallbackId;
  uint64 Id;
  bool bDone = false;
  bool bAbandoned = false;

  FRustealLatentAction(const FLatentActionInfo &Info, FRustealLibrary *InLibrary,
                       uint64 InAbortCallbackId, uint64 InId)
      : ExecutionFunction(Info.ExecutionFunction), OutputLink(Info.Linkage),
        CallbackTarget(Info.CallbackTarget), Library(InLibrary),
        AbortCallbackId(InAbortCallbackId), Id(InId) {
    GLatentActions.Add(Id, this);
  }

  virtual ~FRustealLatentAction() override {
    GLatentActions.Remove(Id);

    RustealCallLibrary(Library, [this](const FRustealRustCallbacks &Cb) {
      Cb.release_delegate_callback(AbortCallbackId);
    });
  }

  virtual void UpdateOperation(FLatentResponse &Response) override {
    if (!Library || bAbandoned) {
      Response.DoneIf(true);
      return;
    }

    Response.FinishAndTriggerIf(bDone, ExecutionFunction, OutputLink,
                                CallbackTarget);
  }

  virtual void NotifyObjectDestroyed() override { Abort(); }

  virtual void NotifyActionAborted() override { Abort(); }

  void Abort() {
    if (bDone) {
      return;
    }

    RustealCallLibrary(Library, [this](const FRustealRustCallbacks &Cb) {
      Cb.invoke_delegate_callback(AbortCallbackId, nullptr);
    });
  }
};
} // namespace

uint64 RustealLatent_FunctionBegin(RustealUObjectHandle Owner,
                                   const uint8 *Info, uint64 AbortCallbackId) {
  UObject *Object = static_cast<UObject *>(Owner.ptr);

  if (!IsValid(Object) || !Info || !GEngine) {
    return 0;
  }

  const FLatentActionInfo &LatentInfo =
      *reinterpret_cast<const FLatentActionInfo *>(Info);

  UWorld *World =
      GEngine->GetWorldFromContextObject(Object, EGetWorldErrorMode::ReturnNull);

  if (!World || !LatentInfo.CallbackTarget) {
    return 0;
  }

  FLatentActionManager &Manager = World->GetLatentActionManager();

  if (Manager.FindExistingAction<FRustealLatentAction>(LatentInfo.CallbackTarget,
                                                       LatentInfo.UUID)) {
    return 0;
  }

  const uint64 Id = GNextLatentAction++;

  Manager.AddNewAction(LatentInfo.CallbackTarget, LatentInfo.UUID,
                       new FRustealLatentAction(LatentInfo, RustealCurrentLibrary(),
                                                AbortCallbackId, Id));

  return Id;
}

void RustealLatent_FunctionFinish(uint64 Id, bool bCompleted) {
  if (FRustealLatentAction **Found = GLatentActions.Find(Id)) {
    (*Found)->bDone = true;
    (*Found)->bAbandoned = !bCompleted;
  }
}

uint32 RustealLatent_LiveFunctions() {
  return static_cast<uint32>(GLatentActions.Num());
}

void RustealLatentFunctionsForgetLibrary(FRustealLibrary *Library) {
  for (const TPair<uint64, FRustealLatentAction *> &Entry : GLatentActions) {
    if (Entry.Value->Library == Library) {
      Entry.Value->Library = nullptr;
    }
  }
}
