#pragma once

#include "CoreMinimal.h"
#include "UObject/ScriptInterface.h"

template <typename InterfaceType>
TScriptInterface<InterfaceType> RustealToScriptInterface(UObject *Object) {
  TScriptInterface<InterfaceType> Result;
  Result.SetObject(Object);

  if (Object) {
    Result.SetInterface(static_cast<InterfaceType *>(Object->GetInterfaceAddress(
        InterfaceType::UClassType::StaticClass())));
  }

  return Result;
}
