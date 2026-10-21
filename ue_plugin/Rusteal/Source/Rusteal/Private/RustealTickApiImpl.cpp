#include "Components/ActorComponent.h"
#include "Containers/Ticker.h"
#include "Engine/Engine.h"
#include "Engine/EngineBaseTypes.h"
#include "Engine/World.h"
#include "GameFramework/Actor.h"
#include "RustealApiTable.h"
#include "RustealErrors.h"
#include "RustealLibrary.h"

struct FRustealTickFunction : public FTickFunction {
  FRustealLibrary *Library = nullptr;
  uint64 CallbackId = 0;
  TWeakObjectPtr<UObject> Target;
  TWeakObjectPtr<UWorld> World;
  bool bRemoved = false;

  virtual ~FRustealTickFunction() override;

  virtual void
  ExecuteTick(float DeltaTime, ELevelTick TickType,
              ENamedThreads::Type CurrentThread,
              const FGraphEventRef &MyCompletionGraphEvent) override;

  virtual FString DiagnosticMessage() override {
    return FString::Printf(TEXT("Rusteal tick function %llu"), CallbackId);
  }

  virtual FName DiagnosticContext(bool bDetailed) override {
    return FName(TEXT("RustealTickFunction"));
  }
};

static TMap<uint64, TUniquePtr<FRustealTickFunction>> GTickFunctions;
static FDelegateHandle GWorldCleanupHandle;
static FTSTicker::FDelegateHandle GCollectHandle;

FRustealTickFunction::~FRustealTickFunction() {
  if (IsTickFunctionRegistered()) {
    UnRegisterTickFunction();
  }

  RustealCallLibrary(Library, [this](const FRustealRustCallbacks &Cb) {
    Cb.release_delegate_callback(CallbackId);
  });
}

void FRustealTickFunction::ExecuteTick(
    float DeltaTime, ELevelTick TickType, ENamedThreads::Type CurrentThread,
    const FGraphEventRef &MyCompletionGraphEvent) {
  if (bRemoved) {
    return;
  }

  if (!Target.IsValid()) {
    bRemoved = true;
    SetTickFunctionEnable(false);
    return;
  }

  float Delta = DeltaTime;

  RustealCallLibrary(Library, [this, &Delta](const FRustealRustCallbacks &Cb) {
    Cb.invoke_delegate_callback(CallbackId, reinterpret_cast<uint8 *>(&Delta));
  });
}

static bool CollectRemoved(float) {
  TArray<uint64> Removed;

  for (const TPair<uint64, TUniquePtr<FRustealTickFunction>> &Entry :
       GTickFunctions) {
    if (Entry.Value->bRemoved) {
      Removed.Add(Entry.Key);
    }
  }

  for (uint64 Id : Removed) {
    GTickFunctions.Remove(Id);
  }

  return true;
}

static void RemoveWorld(UWorld *World, bool, bool) {
  TArray<uint64> Removed;

  for (const TPair<uint64, TUniquePtr<FRustealTickFunction>> &Entry :
       GTickFunctions) {
    UWorld *Owner = Entry.Value->World.Get();

    if (!Owner || Owner == World) {
      Removed.Add(Entry.Key);
    }
  }

  for (uint64 Id : Removed) {
    GTickFunctions.Remove(Id);
  }
}

static void EnsureHooks() {
  if (!GWorldCleanupHandle.IsValid()) {
    GWorldCleanupHandle =
        FWorldDelegates::OnWorldCleanup.AddStatic(&RemoveWorld);
  }

  if (!GCollectHandle.IsValid()) {
    GCollectHandle = FTSTicker::GetCoreTicker().AddTicker(
        FTickerDelegate::CreateStatic(&CollectRemoved));
  }
}

static FTickFunction *PrimaryTickOf(UObject *Object) {
  if (AActor *Actor = Cast<AActor>(Object)) {
    return &Actor->PrimaryActorTick;
  }

  if (UActorComponent *Component = Cast<UActorComponent>(Object)) {
    return &Component->PrimaryComponentTick;
  }

  return nullptr;
}

static FRustealTickFunction *Find(uint64 CallbackId) {
  TUniquePtr<FRustealTickFunction> *Found = GTickFunctions.Find(CallbackId);

  if (!Found || (*Found)->bRemoved) {
    return nullptr;
  }

  return Found->Get();
}

ERustealErrorCode RustealTick_Add(RustealUObjectHandle Target, uint8 Group,
                                  uint64 CallbackId) {
  UObject *Object = static_cast<UObject *>(Target.ptr);

  if (!IsValid(Object) || !GEngine) {
    return RustealFail(ERustealErrorCode::ObjectDestroyed);
  }

  if (Group >= TG_NewlySpawned) {
    return RustealFail(ERustealErrorCode::InvalidOperation,
                       TEXT("add_tick_function: no such tick group"));
  }

  UWorld *World = GEngine->GetWorldFromContextObject(
      Object, EGetWorldErrorMode::ReturnNull);

  if (!World || !World->PersistentLevel) {
    return RustealFail(ERustealErrorCode::ObjectDestroyed,
                       TEXT("add_tick_function: the object has no world"));
  }

  EnsureHooks();

  TUniquePtr<FRustealTickFunction> Function =
      MakeUnique<FRustealTickFunction>();
  Function->Library = RustealCurrentLibrary();
  Function->CallbackId = CallbackId;
  Function->Target = Object;
  Function->World = World;
  Function->bCanEverTick = true;
  Function->bStartWithTickEnabled = true;
  Function->bRunOnAnyThread = false;
  Function->TickGroup = static_cast<ETickingGroup>(Group);
  Function->EndTickGroup = static_cast<ETickingGroup>(Group);
  Function->RegisterTickFunction(World->PersistentLevel);
  Function->SetTickFunctionEnable(true);

  GTickFunctions.Add(CallbackId, MoveTemp(Function));
  return ERustealErrorCode::Ok;
}

void RustealTick_Remove(uint64 CallbackId) {
  if (FRustealTickFunction *Function = Find(CallbackId)) {
    Function->bRemoved = true;
    Function->SetTickFunctionEnable(false);
  }
}

ERustealErrorCode RustealTick_SetEnabled(uint64 CallbackId, bool bEnabled) {
  FRustealTickFunction *Function = Find(CallbackId);

  if (!Function) {
    return RustealFail(ERustealErrorCode::ObjectDestroyed,
                       TEXT("the tick function was removed"));
  }

  Function->SetTickFunctionEnable(bEnabled);
  return ERustealErrorCode::Ok;
}

ERustealErrorCode RustealTick_SetInterval(uint64 CallbackId, float Seconds) {
  FRustealTickFunction *Function = Find(CallbackId);

  if (!Function) {
    return RustealFail(ERustealErrorCode::ObjectDestroyed,
                       TEXT("the tick function was removed"));
  }

  Function->UpdateTickIntervalAndCoolDown(FMath::Max(Seconds, 0.0f));
  return ERustealErrorCode::Ok;
}

ERustealErrorCode RustealTick_AddPrerequisite(uint64 CallbackId,
                                              RustealUObjectHandle Object,
                                              uint64 OtherCallbackId) {
  FRustealTickFunction *Function = Find(CallbackId);

  if (!Function) {
    return RustealFail(ERustealErrorCode::ObjectDestroyed,
                       TEXT("the tick function was removed"));
  }

  if (OtherCallbackId != 0) {
    FRustealTickFunction *Other = Find(OtherCallbackId);

    if (!Other || !Other->Target.IsValid()) {
      return RustealFail(ERustealErrorCode::ObjectDestroyed,
                         TEXT("the other tick function was removed"));
    }

    Function->AddPrerequisite(Other->Target.Get(), *Other);
    return ERustealErrorCode::Ok;
  }

  UObject *Target = static_cast<UObject *>(Object.ptr);

  if (!IsValid(Target)) {
    return RustealFail(ERustealErrorCode::ObjectDestroyed);
  }

  FTickFunction *Primary = PrimaryTickOf(Target);

  if (!Primary) {
    return RustealFail(
        ERustealErrorCode::InvalidOperation,
        TEXT("a tick function waits for an actor or an actor component"));
  }

  Function->AddPrerequisite(Target, *Primary);
  return ERustealErrorCode::Ok;
}

uint32 RustealTick_LiveFunctions() {
  uint32 Live = 0;

  for (const TPair<uint64, TUniquePtr<FRustealTickFunction>> &Entry :
       GTickFunctions) {
    Live += Entry.Value->bRemoved ? 0 : 1;
  }

  return Live;
}

void RustealTickFunctionsForgetLibrary(FRustealLibrary *Library) {
  TArray<uint64> Owned;

  for (const TPair<uint64, TUniquePtr<FRustealTickFunction>> &Entry :
       GTickFunctions) {
    if (Entry.Value->Library == Library) {
      Entry.Value->Library = nullptr;
      Owned.Add(Entry.Key);
    }
  }

  for (uint64 Id : Owned) {
    GTickFunctions.Remove(Id);
  }
}
