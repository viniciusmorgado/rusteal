#pragma once

#include "CoreMinimal.h"
#include "Net/Core/PushModel/PushModelMacros.h"
#include "UObject/Object.h"
#include "RustealReplicatedObject.generated.h"

/**
 * A plain object that replicates as a subobject of the actor it is registered
 * with: its replicated properties reach the clients, and its remote functions
 * go through that actor's connection.
 */
UCLASS(Abstract, Blueprintable)
class RUSTEAL_API URustealReplicatedObject : public UObject {
  GENERATED_BODY()

  REPLICATED_BASE_CLASS(URustealReplicatedObject)

public:
  virtual bool IsSupportedForNetworking() const override { return true; }

  virtual void GetLifetimeReplicatedProps(
      TArray<FLifetimeProperty> &OutLifetimeProps) const override;

  virtual int32 GetFunctionCallspace(UFunction *Function,
                                     FFrame *Stack) override;

  virtual bool CallRemoteFunction(UFunction *Function, void *Parms,
                                  FOutParmRec *OutParms,
                                  FFrame *Stack) override;

  virtual UWorld *GetWorld() const override;
};
