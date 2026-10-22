#include "GameFramework/Actor.h"
#include "Net/Core/PushModel/PushModel.h"
#include "RustealApiTable.h"
#include "RustealErrors.h"
#include "RustealStrings.h"
#include "UObject/CoreNet.h"
#include "UObject/UnrealType.h"

void RustealNet_RpcValidateFailed(const uint8 *Reason, uint32 ReasonLen) {
  RPC_ValidateFailed(*RustealUtf8ToFString(Reason, ReasonLen));
}

void RustealNet_MarkPropertyDirty(RustealUObjectHandle Object,
                                  RustealFPropertyHandle Prop) {
#if WITH_PUSH_MODEL
  UObject *Owner = static_cast<UObject *>(Object.ptr);
  FProperty *Property = static_cast<FProperty *>(Prop.ptr);

  if (!IS_PUSH_MODEL_ENABLED() || !::IsValid(Owner) || !Property ||
      !Property->HasAnyPropertyFlags(CPF_Net) ||
      !Owner->GetClass()->HasAnyClassFlags(CLASS_ReplicationDataIsSetUp)) {
    return;
  }

  MARK_PROPERTY_DIRTY_UNSAFE(Owner, Property->RepIndex);
#endif
}

ERustealErrorCode RustealNet_AddReplicatedSubobject(RustealUObjectHandle Actor,
                                                    RustealUObjectHandle Object,
                                                    uint8 Condition) {
  AActor *Owner = Cast<AActor>(static_cast<UObject *>(Actor.ptr));
  UObject *Subobject = static_cast<UObject *>(Object.ptr);

  if (!::IsValid(Owner) || !::IsValid(Subobject)) {
    return RustealFail(ERustealErrorCode::ObjectDestroyed,
                       TEXT("replicating a subobject: the actor or the "
                            "object is gone"));
  }

  if (!Subobject->IsSupportedForNetworking()) {
    return RustealFail(
        ERustealErrorCode::InvalidOperation,
        FString::Printf(TEXT("%s does not replicate: a replicated subobject "
                             "derives from RustealReplicatedObject"),
                        *RustealDescribe(Subobject)));
  }

  if (Condition >= COND_Max) {
    return RustealFail(
        ERustealErrorCode::InvalidOperation,
        FString::Printf(TEXT("%d is not a replication condition"), Condition));
  }

  if (!Owner->IsUsingRegisteredSubObjectList()) {
    static FBoolProperty *Flag = FindFProperty<FBoolProperty>(
        AActor::StaticClass(), TEXT("bReplicateUsingRegisteredSubObjectList"));

    if (Flag) {
      Flag->SetPropertyValue_InContainer(Owner, true);
    }
  }

  Owner->AddReplicatedSubObject(Subobject,
                                static_cast<ELifetimeCondition>(Condition));
  return ERustealErrorCode::Ok;
}

ERustealErrorCode
RustealNet_RemoveReplicatedSubobject(RustealUObjectHandle Actor,
                                     RustealUObjectHandle Object) {
  AActor *Owner = Cast<AActor>(static_cast<UObject *>(Actor.ptr));
  UObject *Subobject = static_cast<UObject *>(Object.ptr);

  if (!::IsValid(Owner)) {
    return RustealFail(ERustealErrorCode::ObjectDestroyed);
  }

  if (Subobject) {
    Owner->RemoveReplicatedSubObject(Subobject);
  }

  return ERustealErrorCode::Ok;
}
