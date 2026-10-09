#pragma once
#include "UObject/NameTypes.h"

static inline FName RustealUnpackFName(uint64_t Packed) {
  FNameEntryId CompIdx =
      FNameEntryId::FromUnstableInt(static_cast<uint32>(Packed & 0xFFFFFFFF));

  int32 Number = static_cast<int32>(Packed >> 32);
  return FName(CompIdx, CompIdx, Number);
}

static inline uint64_t RustealPackFName(const FName &Name) {
  return static_cast<uint64>(Name.GetComparisonIndex().ToUnstableInt()) |
         (static_cast<uint64>(Name.GetNumber()) << 32);
}
