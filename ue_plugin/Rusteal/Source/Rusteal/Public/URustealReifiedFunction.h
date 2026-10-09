#pragma once

#include "UObject/ObjectMacros.h"
#include "URustealReifiedFunction.generated.h"

struct FRustealLibrary;

UCLASS()
class URustealReifiedFunction : public UFunction {
  GENERATED_BODY()

public:
  uint64 CallbackId = 0;

  FRustealLibrary *Library = nullptr;

  DECLARE_FUNCTION(execCallRustFunction);
};
