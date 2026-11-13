#pragma once

#include "CoreMinimal.h"
#include "UObject/Class.h"
#include "UObject/UObjectGlobals.h"

inline UScriptStruct *RustealFindScriptStruct(const TCHAR *Path) {
  return FindObject<UScriptStruct>(nullptr, Path);
}
