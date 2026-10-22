#include "HAL/LowLevelMemTracker.h"
#include "ProfilingDebugging/CpuProfilerTrace.h"
#include "RustealApiTable.h"
#include "RustealStrings.h"
#include "Stats/DynamicStats.h"
#include "Stats/Stats.h"

DECLARE_STATS_GROUP(TEXT("Rusteal"), STATGROUP_Rusteal, STATCAT_Advanced);

uint32 RustealProfiling_TraceEventType(const uint8 *Name, uint32 NameLen) {
#if CPUPROFILERTRACE_ENABLED
  return FCpuProfilerTrace::OutputEventType(
      *RustealUtf8ToFString(Name, NameLen));
#else
  return 0;
#endif
}

void RustealProfiling_TraceBeginEvent(uint32 EventType) {
#if CPUPROFILERTRACE_ENABLED
  if (EventType != 0) {
    FCpuProfilerTrace::OutputBeginEvent(EventType);
  }
#endif
}

void RustealProfiling_TraceEndEvent() {
#if CPUPROFILERTRACE_ENABLED
  FCpuProfilerTrace::OutputEndEvent();
#endif
}

uint64 RustealProfiling_StatCreate(const uint8 *Name, uint32 NameLen) {
#if STATS
  const TStatId Stat =
      FDynamicStats::CreateStatId<FStatGroup_STATGROUP_Rusteal>(
          RustealUtf8ToFString(Name, NameLen));

  return reinterpret_cast<uint64>(Stat.GetRawPointer());
#else
  return 0;
#endif
}

uint64 RustealProfiling_StatBegin(uint64 Stat) {
#if STATS
  if (Stat == 0) {
    return 0;
  }

  FScopeCycleCounter *Counter = new FScopeCycleCounter(
      TStatId(reinterpret_cast<const TStatIdData *>(Stat)));

  return reinterpret_cast<uint64>(Counter);
#else
  return 0;
#endif
}

void RustealProfiling_StatEnd(uint64 Counter) {
#if STATS
  delete reinterpret_cast<FScopeCycleCounter *>(Counter);
#endif
}

uint64 RustealProfiling_MemoryScopeBegin(const uint8 *Tag, uint32 TagLen) {
#if ENABLE_LOW_LEVEL_MEM_TRACKER
  if (!FLowLevelMemTracker::IsEnabled()) {
    return 0;
  }

  FLLMScope *Scope =
      new FLLMScope(FName(*RustealUtf8ToFString(Tag, TagLen)), false,
                    ELLMTagSet::None, ELLMTracker::Default);

  return reinterpret_cast<uint64>(Scope);
#else
  return 0;
#endif
}

void RustealProfiling_MemoryScopeEnd(uint64 Scope) {
#if ENABLE_LOW_LEVEL_MEM_TRACKER
  delete reinterpret_cast<FLLMScope *>(Scope);
#endif
}
