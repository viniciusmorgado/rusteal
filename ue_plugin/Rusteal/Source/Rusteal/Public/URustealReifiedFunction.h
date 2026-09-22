#pragma once

#include "CoreMinimal.h"
#include "UObject/ObjectMacros.h"
#include "URustealReifiedFunction.generated.h"

// A UFunction created at runtime by Rust via the Reify API.
// When UE calls this function (via ProcessEvent or Blueprint VM),
// it dispatches to the registered Rust callback.
UCLASS()
class URustealReifiedFunction : public UFunction
{
    GENERATED_BODY()

public:
    // Rust-side callback ID for dispatching to the correct Rust function.
    uint64 CallbackId = 0;

    // Native thunk called by the Blueprint VM.
    DECLARE_FUNCTION(execCallRustFunction);
};
