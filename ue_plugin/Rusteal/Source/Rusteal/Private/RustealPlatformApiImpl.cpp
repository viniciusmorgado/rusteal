#include "HAL/PlatformProcess.h"
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

FRustealPlatformApi GPlatformApi = {
    &LaunchTaskImpl,
    &SecondsImpl,
    &SleepImpl,
    &IsGameThreadImpl,
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
