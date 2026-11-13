#include "Engine/Level.h"
#include "Engine/World.h"
#include "GameFramework/GameModeBase.h"
#include "Misc/CoreDelegates.h"
#include "Navigation/PathFollowingComponent.h"
#include "RustealApiTable.h"
#include "RustealErrors.h"
#include "RustealLibrary.h"

namespace {
struct FRustealNativeBinding {
  FRustealLibrary *Library = nullptr;
  ERustealNativeEvent Kind = ERustealNativeEvent::PreExit;
  FDelegateHandle Handle;
  TWeakObjectPtr<UObject> Target;
};

TMap<uint64, FRustealNativeBinding> GNativeBindings;

TFunction<void(FRustealNativeEventArgs)> Fire(FRustealLibrary *Library,
                                              uint64 CallbackId) {
  return [Library, CallbackId](FRustealNativeEventArgs Args) {
    RustealCallLibrary(Library, [CallbackId, &Args](const FRustealRustCallbacks &Cb) {
      Cb.invoke_delegate_callback(CallbackId, reinterpret_cast<uint8 *>(&Args));
    });
  };
}

FRustealNativeEventArgs Args(UObject *First = nullptr, UObject *Second = nullptr,
                             uint32 A = 0, uint32 B = 0) {
  FRustealNativeEventArgs Out;
  Out.first = RustealUObjectHandle{First};
  Out.second = RustealUObjectHandle{Second};
  Out.a = A;
  Out.b = B;
  return Out;
}

TMulticastDelegate<void()> *CoreEvent(ERustealNativeEvent Kind) {
  switch (Kind) {
  case ERustealNativeEvent::PreExit:
    return &FCoreDelegates::OnPreExit;
  case ERustealNativeEvent::WillEnterBackground:
    return &FCoreDelegates::ApplicationWillEnterBackgroundDelegate;
  case ERustealNativeEvent::HasEnteredForeground:
    return &FCoreDelegates::ApplicationHasEnteredForegroundDelegate;
  case ERustealNativeEvent::WillDeactivate:
    return &FCoreDelegates::ApplicationWillDeactivateDelegate;
  case ERustealNativeEvent::HasReactivated:
    return &FCoreDelegates::ApplicationHasReactivatedDelegate;
  default:
    return nullptr;
  }
}

void Unbind(const FRustealNativeBinding &Binding) {
  if (TMulticastDelegate<void()> *Core = CoreEvent(Binding.Kind)) {
    Core->Remove(Binding.Handle);
    return;
  }

  switch (Binding.Kind) {
  case ERustealNativeEvent::WorldInitialized:
    FWorldDelegates::OnPostWorldInitialization.Remove(Binding.Handle);
    break;
  case ERustealNativeEvent::WorldCleanup:
    FWorldDelegates::OnWorldCleanup.Remove(Binding.Handle);
    break;
  case ERustealNativeEvent::LevelAdded:
    FWorldDelegates::LevelAddedToWorld.Remove(Binding.Handle);
    break;
  case ERustealNativeEvent::LevelRemoved:
    FWorldDelegates::LevelRemovedFromWorld.Remove(Binding.Handle);
    break;
  case ERustealNativeEvent::PostLogin:
    FGameModeEvents::OnGameModePostLoginEvent().Remove(Binding.Handle);
    break;
  case ERustealNativeEvent::Logout:
    FGameModeEvents::OnGameModeLogoutEvent().Remove(Binding.Handle);
    break;
  case ERustealNativeEvent::MoveRequestFinished:
    if (UPathFollowingComponent *Component =
            Cast<UPathFollowingComponent>(Binding.Target.Get())) {
      Component->OnRequestFinished.Remove(Binding.Handle);
    }
    break;
  default:
    break;
  }
}
} // namespace

ERustealErrorCode RustealEvents_Bind(uint32 KindValue,
                                     RustealUObjectHandle TargetHandle,
                                     uint64 CallbackId) {
  const ERustealNativeEvent Kind = static_cast<ERustealNativeEvent>(KindValue);
  FRustealLibrary *Library = RustealCurrentLibrary();
  TFunction<void(FRustealNativeEventArgs)> Call = Fire(Library, CallbackId);

  FRustealNativeBinding Binding;
  Binding.Library = Library;
  Binding.Kind = Kind;

  if (TMulticastDelegate<void()> *Core = CoreEvent(Kind)) {
    Binding.Handle = Core->AddLambda([Call]() { Call(Args()); });
  } else {
    switch (Kind) {
    case ERustealNativeEvent::WorldInitialized:
      Binding.Handle = FWorldDelegates::OnPostWorldInitialization.AddLambda(
          [Call](UWorld *World, const UWorld::InitializationValues) {
            Call(Args(World));
          });
      break;
    case ERustealNativeEvent::WorldCleanup:
      Binding.Handle = FWorldDelegates::OnWorldCleanup.AddLambda(
          [Call](UWorld *World, bool bSessionEnded, bool bCleanupResources) {
            Call(Args(World, nullptr, bSessionEnded, bCleanupResources));
          });
      break;
    case ERustealNativeEvent::LevelAdded:
      Binding.Handle = FWorldDelegates::LevelAddedToWorld.AddLambda(
          [Call](ULevel *Level, UWorld *World) { Call(Args(Level, World)); });
      break;
    case ERustealNativeEvent::LevelRemoved:
      Binding.Handle = FWorldDelegates::LevelRemovedFromWorld.AddLambda(
          [Call](ULevel *Level, UWorld *World) { Call(Args(Level, World)); });
      break;
    case ERustealNativeEvent::PostLogin:
      Binding.Handle = FGameModeEvents::OnGameModePostLoginEvent().AddLambda(
          [Call](AGameModeBase *GameMode, APlayerController *Player) {
            Call(Args(GameMode, Player));
          });
      break;
    case ERustealNativeEvent::Logout:
      Binding.Handle = FGameModeEvents::OnGameModeLogoutEvent().AddLambda(
          [Call](AGameModeBase *GameMode, AController *Exiting) {
            Call(Args(GameMode, Exiting));
          });
      break;
    case ERustealNativeEvent::MoveRequestFinished: {
      UPathFollowingComponent *Component = Cast<UPathFollowingComponent>(
          static_cast<UObject *>(TargetHandle.ptr));

      if (!IsValid(Component)) {
        return RustealFail(ERustealErrorCode::ObjectDestroyed);
      }

      Binding.Target = Component;
      Binding.Handle = Component->OnRequestFinished.AddLambda(
          [Call](FAIRequestID RequestId, const FPathFollowingResult &Result) {
            Call(Args(nullptr, nullptr, RequestId.GetID(),
                      static_cast<uint32>(Result.Code.GetValue()) |
                          (static_cast<uint32>(Result.Flags) << 8)));
          });
      break;
    }
    default:
      return RustealFail(ERustealErrorCode::InvalidOperation,
                         TEXT("bind_native_event: unknown event"));
    }
  }

  GNativeBindings.Add(CallbackId, Binding);
  return ERustealErrorCode::Ok;
}

ERustealErrorCode RustealEvents_Unbind(uint64 CallbackId) {
  FRustealNativeBinding Binding;

  if (GNativeBindings.RemoveAndCopyValue(CallbackId, Binding)) {
    Unbind(Binding);
  }

  return ERustealErrorCode::Ok;
}

void RustealEventsForgetLibrary(FRustealLibrary *Library) {
  TArray<uint64> Owned;

  for (const TPair<uint64, FRustealNativeBinding> &Entry : GNativeBindings) {
    if (Entry.Value.Library == Library) {
      Owned.Add(Entry.Key);
    }
  }

  for (uint64 Id : Owned) {
    RustealEvents_Unbind(Id);
  }
}
