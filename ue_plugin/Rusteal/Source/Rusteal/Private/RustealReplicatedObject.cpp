#include "RustealReplicatedObject.h"

#include "Engine/BlueprintGeneratedClass.h"
#include "Engine/NetDriver.h"
#include "GameFramework/Actor.h"
#include "Net/UnrealNetwork.h"

void URustealReplicatedObject::GetLifetimeReplicatedProps(
    TArray<FLifetimeProperty> &OutLifetimeProps) const {
  Super::GetLifetimeReplicatedProps(OutLifetimeProps);

  if (const UBlueprintGeneratedClass *Generated =
          Cast<UBlueprintGeneratedClass>(GetClass())) {
    Generated->GetLifetimeBlueprintReplicationList(OutLifetimeProps);
  }
}

int32 URustealReplicatedObject::GetFunctionCallspace(UFunction *Function,
                                                     FFrame *Stack) {
  if (AActor *Owner = GetTypedOuter<AActor>()) {
    return Owner->GetFunctionCallspace(Function, Stack);
  }

  return Super::GetFunctionCallspace(Function, Stack);
}

bool URustealReplicatedObject::CallRemoteFunction(UFunction *Function,
                                                  void *Parms,
                                                  FOutParmRec *OutParms,
                                                  FFrame *Stack) {
  AActor *Owner = GetTypedOuter<AActor>();
  UNetDriver *Driver = Owner ? Owner->GetNetDriver() : nullptr;

  if (!Driver) {
    return false;
  }

  Driver->ProcessRemoteFunction(Owner, Function, Parms, OutParms, Stack, this);
  return true;
}

UWorld *URustealReplicatedObject::GetWorld() const {
  if (HasAnyFlags(RF_ClassDefaultObject)) {
    return nullptr;
  }

  const AActor *Owner = GetTypedOuter<AActor>();
  return Owner ? Owner->GetWorld() : nullptr;
}
