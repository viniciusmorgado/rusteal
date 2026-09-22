// Dummy UCLASS to ensure RustealGenerator appears in the UHT manifest.
// The RustealExporter UBT plugin requires its ModuleName to be present in the manifest
// so that MakePath() can determine the output directory.

#pragma once

#include "CoreMinimal.h"
#include "UObject/Object.h"
#include "RustealGeneratorDummy.generated.h"

UCLASS(NotBlueprintable, Hidden)
class URustealGeneratorDummy : public UObject
{
	GENERATED_BODY()
};
