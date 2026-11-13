#include "HAL/PlatformMisc.h"
#include "HAL/PlatformProcess.h"
#include "HAL/Runnable.h"
#include "HAL/RunnableThread.h"
#include "HAL/PlatformTime.h"
#include "RustealApiTable.h"
#include "Tasks/Task.h"

using FRustealTaskRun = void (*)(void *);

static bool LaunchTaskImpl(const void *Run, uint8 *Data) {
  if (!Run) {
    return false;
  }

  const FRustealTaskRun Function =
      reinterpret_cast<FRustealTaskRun>(const_cast<void *>(Run));

  UE::Tasks::Launch(
      TEXT("Rusteal task::spawn"), [Function, Data]() { Function(Data); },
      UE::Tasks::ETaskPriority::BackgroundNormal);

  return true;
}

static double SecondsImpl() { return FPlatformTime::Seconds(); }

static void SleepImpl(float Seconds) { FPlatformProcess::Sleep(Seconds); }

static bool IsGameThreadImpl() { return IsInGameThread(); }

class FRustealThread final : public FRunnable {
public:
  FRustealThread(FRustealTaskRun InFunction, uint8 *InData)
      : Function(InFunction), Data(InData) {}

  virtual uint32 Run() override {
    Function(Data);
    return 0;
  }

  FRunnableThread *Thread = nullptr;

private:
  FRustealTaskRun Function;
  uint8 *Data;
};

static uint8 *CreateThreadImpl(const void *Run, uint8 *Data, const uint8 *Name,
                               uint32 NameLen) {
  if (!Run) {
    return nullptr;
  }

  const FUTF8ToTCHAR ThreadName(reinterpret_cast<const ANSICHAR *>(Name),
                                NameLen);

  FRustealThread *Runnable = new FRustealThread(
      reinterpret_cast<FRustealTaskRun>(const_cast<void *>(Run)), Data);

  Runnable->Thread = FRunnableThread::Create(
      Runnable, *FString(ThreadName.Length(), ThreadName.Get()), 0,
      TPri_Normal);

  if (!Runnable->Thread) {
    delete Runnable;
    return nullptr;
  }

  return reinterpret_cast<uint8 *>(Runnable);
}

static void JoinThreadImpl(uint8 *Handle) {
  FRustealThread *Runnable = reinterpret_cast<FRustealThread *>(Handle);

  if (!Runnable) {
    return;
  }

  Runnable->Thread->WaitForCompletion();
  delete Runnable->Thread;
  delete Runnable;
}

static uint32 WorkerCountImpl() {
  return static_cast<uint32>(
      FMath::Max(1, FPlatformMisc::NumberOfWorkerThreadsToSpawn()));
}

FRustealPlatformApi GPlatformApi = {
    &LaunchTaskImpl,   &SecondsImpl,    &SleepImpl,       &IsGameThreadImpl,
    &CreateThreadImpl, &JoinThreadImpl, &WorkerCountImpl,
};

#if RUSTEAL_STATIC
extern "C" void *rusteal_engine_malloc(SIZE_T Size, uint32 Alignment) {
  return FMemory::Malloc(Size, Alignment);
}

extern "C" void *rusteal_engine_malloc_zeroed(SIZE_T Size, uint32 Alignment) {
  return FMemory::MallocZeroed(Size, Alignment);
}

extern "C" void *rusteal_engine_realloc(void *Original, SIZE_T Size,
                                        uint32 Alignment) {
  return FMemory::Realloc(Original, Size, Alignment);
}

extern "C" void rusteal_engine_free(void *Original) { FMemory::Free(Original); }
#endif
