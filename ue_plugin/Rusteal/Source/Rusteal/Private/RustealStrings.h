#pragma once

#include "Containers/StringConv.h"
#include "CoreMinimal.h"

inline FString RustealUtf8ToFString(const void *Bytes, uint32 Len) {
  if (!Bytes || Len == 0) {
    return FString();
  }

  const auto Converted = StringCast<TCHAR>(static_cast<const UTF8CHAR *>(Bytes),
                                           static_cast<int32>(Len));

  return FString(Converted.Length(), Converted.Get());
}
