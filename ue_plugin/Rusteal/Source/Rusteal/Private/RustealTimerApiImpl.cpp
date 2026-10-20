#include "Engine/Engine.h"
#include "Engine/World.h"
#include "RustealApiTable.h"
#include "RustealErrors.h"
#include "RustealLibrary.h"
#include "TimerManager.h"

static_assert(sizeof(FTimerHandle) == sizeof(uint64),
              "FTimerHandle crosses to Rust as a uint64");

struct FRustealTimerCallback {
  FRustealLibrary *Library = nullptr;
  uint64 CallbackId = 0;
  TWeakObjectPtr<UWorld> World;
  FTimerHandle Handle;

  ~FRustealTimerCallback();
};

static TSet<FRustealTimerCallback *> GTimerCallbacks;

FRustealTimerCallback::~FRustealTimerCallback() {
  GTimerCallbacks.Remove(this);

  RustealCallLibrary(Library, [this](const FRustealRustCallbacks &Cb) {
    Cb.release_delegate_callback(CallbackId);
  });
}

static uint64 PackHandle(const FTimerHandle &Handle) {
  uint64 Packed = 0;
  FMemory::Memcpy(&Packed, &Handle, sizeof(Packed));
  return Packed;
}

static FTimerHandle UnpackHandle(uint64 Packed) {
  FTimerHandle Handle;
  FMemory::Memcpy(&Handle, &Packed, sizeof(Packed));
  return Handle;
}

static UWorld *ContextWorld(RustealUObjectHandle Context) {
  UObject *Object = static_cast<UObject *>(Context.ptr);

  if (!Object) {
    return GWorld;
  }

  if (!IsValid(Object) || !GEngine) {
    return nullptr;
  }

  return GEngine->GetWorldFromContextObject(Object,
                                             EGetWorldErrorMode::ReturnNull);
}

static FTimerDelegate MakeTimerDelegate(UWorld *World, uint64 CallbackId,
                                        TSharedPtr<FRustealTimerCallback> &Out) {
  TSharedPtr<FRustealTimerCallback> Callback =
      MakeShared<FRustealTimerCallback>();

  Callback->Library = RustealCurrentLibrary();
  Callback->CallbackId = CallbackId;
  Callback->World = World;
  GTimerCallbacks.Add(Callback.Get());
  Out = Callback;

  return FTimerDelegate::CreateLambda([Callback]() {
    RustealCallLibrary(Callback->Library,
                       [&Callback](const FRustealRustCallbacks &Cb) {
                         Cb.invoke_delegate_callback(Callback->CallbackId,
                                                     nullptr);
                       });
  });
}

ERustealErrorCode RustealTimer_SetTimer(RustealUObjectHandle Context, float Rate,
                                        bool bLooping, bool bMaxOncePerFrame,
                                        float FirstDelay, uint64 CallbackId,
                                        uint64 *OutHandle) {
  UWorld *World = ContextWorld(Context);

  if (!World) {
    return RustealFail(ERustealErrorCode::ObjectDestroyed,
                       TEXT("set_timer: the context object has no world"));
  }

  TSharedPtr<FRustealTimerCallback> Callback;
  FTimerDelegate Delegate = MakeTimerDelegate(World, CallbackId, Callback);

  FTimerHandle Handle;
  FTimerManagerTimerParameters Parameters;
  Parameters.bLoop = bLooping;
  Parameters.bMaxOncePerFrame = bMaxOncePerFrame;
  Parameters.FirstDelay = FirstDelay;

  World->GetTimerManager().SetTimer(Handle, Delegate, Rate, Parameters);

  if (!Handle.IsValid()) {
    return RustealFail(ERustealErrorCode::InvalidOperation,
                       TEXT("set_timer: a timer runs for more than 0 seconds"));
  }

  Callback->Handle = Handle;

  if (OutHandle) {
    *OutHandle = PackHandle(Handle);
  }

  return ERustealErrorCode::Ok;
}

ERustealErrorCode RustealTimer_SetTimerForNextTick(RustealUObjectHandle Context,
                                                 uint64 CallbackId,
                                                 uint64 *OutHandle) {
  UWorld *World = ContextWorld(Context);

  if (!World) {
    return RustealFail(ERustealErrorCode::ObjectDestroyed,
                       TEXT("set_timer: the context object has no world"));
  }

  TSharedPtr<FRustealTimerCallback> Callback;
  FTimerDelegate Delegate = MakeTimerDelegate(World, CallbackId, Callback);

  const FTimerHandle Handle =
      World->GetTimerManager().SetTimerForNextTick(MoveTemp(Delegate));

  Callback->Handle = Handle;

  if (OutHandle) {
    *OutHandle = PackHandle(Handle);
  }

  return ERustealErrorCode::Ok;
}

ERustealErrorCode RustealTimer_ClearTimer(RustealUObjectHandle Context,
                                        uint64 Packed) {
  UWorld *World = ContextWorld(Context);

  if (!World) {
    return ERustealErrorCode::Ok;
  }

  FTimerHandle Handle = UnpackHandle(Packed);
  World->GetTimerManager().ClearTimer(Handle);
  return ERustealErrorCode::Ok;
}

ERustealErrorCode RustealTimer_PauseTimer(RustealUObjectHandle Context,
                                        uint64 Packed, bool bPaused) {
  UWorld *World = ContextWorld(Context);

  if (!World) {
    return RustealFail(ERustealErrorCode::ObjectDestroyed);
  }

  if (bPaused) {
    World->GetTimerManager().PauseTimer(UnpackHandle(Packed));
  } else {
    World->GetTimerManager().UnPauseTimer(UnpackHandle(Packed));
  }

  return ERustealErrorCode::Ok;
}

float RustealTimer_TimerRemaining(RustealUObjectHandle Context, uint64 Packed) {
  UWorld *World = ContextWorld(Context);

  if (!World) {
    return -1.0f;
  }

  return World->GetTimerManager().GetTimerRemaining(UnpackHandle(Packed));
}

uint32 RustealTimer_LiveTimerCallbacks() {
  return static_cast<uint32>(GTimerCallbacks.Num());
}

void RustealTimersForgetLibrary(FRustealLibrary *Library) {
  TArray<FRustealTimerCallback *> Owned;

  for (FRustealTimerCallback *Callback : GTimerCallbacks) {
    if (Callback->Library == Library) {
      Owned.Add(Callback);
    }
  }

  for (FRustealTimerCallback *Callback : Owned) {
    Callback->Library = nullptr;
  }

  for (FRustealTimerCallback *Callback : Owned) {
    if (!GTimerCallbacks.Contains(Callback)) {
      continue;
    }

    if (UWorld *World = Callback->World.Get()) {
      FTimerHandle Handle = Callback->Handle;
      World->GetTimerManager().ClearTimer(Handle);
    }
  }
}
