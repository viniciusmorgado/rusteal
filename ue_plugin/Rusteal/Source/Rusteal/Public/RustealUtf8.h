#pragma once

#include "CoreMinimal.h"

RUSTEAL_API void RustealWriteUtf8(const FString &Value, void *Buf,
                                  uint32 BufLen, uint32 *OutLen);
