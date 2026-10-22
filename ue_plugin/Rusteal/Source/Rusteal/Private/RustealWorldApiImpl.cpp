#include "Engine/DataTable.h"
#include "RustealErrors.h"
#include "Engine/EngineTypes.h"
#include "Engine/World.h"
#include "EngineUtils.h"
#include "GameFramework/Actor.h"
#include "GameFramework/Pawn.h"
#include "RustealApiTable.h"
#include "UObject/UObjectGlobals.h"
#include "RustealStrings.h"

static FString Utf8ToFStr(const uint8 *Buf, uint32 Len) {
  return RustealUtf8ToFString(Buf, Len);
}

static RustealUObjectHandle SpawnActorImpl(RustealUObjectHandle WorldHandle,
                                           RustealUClassHandle ClsHandle,
                                           const uint8 *TransformBuf,
                                           uint32 TransformSize,
                                           RustealUObjectHandle OwnerHandle) {
  UWorld *World = Cast<UWorld>(static_cast<UObject *>(WorldHandle.ptr));
  UClass *Class = static_cast<UClass *>(ClsHandle.ptr);

  if (!World || !Class) {
    return RustealUObjectHandle{nullptr};
  }

  FTransform SpawnTransform = FTransform::Identity;

  if (TransformBuf && TransformSize > 0) {
    const uint32 CopySize =
        FMath::Min(TransformSize, static_cast<uint32>(sizeof(FTransform)));

    FMemory::Memcpy(&SpawnTransform, TransformBuf, CopySize);
  }

  FActorSpawnParameters Params;
  AActor *Owner = Cast<AActor>(static_cast<UObject *>(OwnerHandle.ptr));

  if (Owner) {
    Params.Owner = Owner;
  }

  AActor *Spawned = World->SpawnActor(Class, &SpawnTransform, Params);
  return RustealUObjectHandle{Spawned};
}

static ERustealErrorCode
GetAllActorsOfClassImpl(RustealUObjectHandle WorldHandle,
                        RustealUClassHandle ClsHandle, uint8 *OutBuf,
                        uint32 BufByteSize, uint32 *OutCount) {
  UWorld *World = Cast<UWorld>(static_cast<UObject *>(WorldHandle.ptr));
  UClass *Class = static_cast<UClass *>(ClsHandle.ptr);

  if (!World || !Class) {
    if (OutCount)
      *OutCount = 0;

    return RustealFail(ERustealErrorCode::NullArgument);
  }

  const uint32 BufCapacity =
      BufByteSize / static_cast<uint32>(sizeof(RustealUObjectHandle));

  RustealUObjectHandle *HandleBuf =
      reinterpret_cast<RustealUObjectHandle *>(OutBuf);

  uint32 Count = 0;

  for (TActorIterator<AActor> It(World, Class); It; ++It) {
    if (HandleBuf && Count < BufCapacity) {
      HandleBuf[Count] = RustealUObjectHandle{*It};
    }

    ++Count;
  }

  if (OutCount)
    *OutCount = Count;

  return ERustealErrorCode::Ok;
}

static RustealUObjectHandle FindObjectImpl(RustealUClassHandle ClsHandle,
                                           const uint8 *PathUtf8,
                                           uint32 PathLen) {
  UClass *Class = static_cast<UClass *>(ClsHandle.ptr);
  const FString Path = Utf8ToFStr(PathUtf8, PathLen);

  UObject *Found = StaticFindObject(Class, nullptr, *Path);
  return RustealUObjectHandle{Found};
}

static RustealUObjectHandle LoadObjectImpl(RustealUClassHandle ClsHandle,
                                           const uint8 *PathUtf8,
                                           uint32 PathLen) {
  UClass *Class = static_cast<UClass *>(ClsHandle.ptr);

  if (!Class)
    Class = UObject::StaticClass();

  const FString Path = Utf8ToFStr(PathUtf8, PathLen);

  UObject *Loaded = StaticLoadObject(Class, nullptr, *Path);
  return RustealUObjectHandle{Loaded};
}

static RustealUObjectHandle GetWorldImpl(RustealUObjectHandle ObjectHandle) {
  UObject *Object = static_cast<UObject *>(ObjectHandle.ptr);

  if (!IsValid(Object))
    return RustealUObjectHandle{nullptr};

  return RustealUObjectHandle{Object->GetWorld()};
}

static RustealUObjectHandle NewObjectImpl(RustealUObjectHandle OuterHandle,
                                          RustealUClassHandle ClsHandle) {
  UClass *Class = static_cast<UClass *>(ClsHandle.ptr);

  if (!Class)
    return RustealUObjectHandle{nullptr};

  UObject *Outer = static_cast<UObject *>(OuterHandle.ptr);

  if (!Outer)
    Outer = GetTransientPackage();

  UObject *Obj = NewObject<UObject>(Outer, Class);
  return RustealUObjectHandle{Obj};
}

static RustealUObjectHandle
SpawnActorDeferredImpl(RustealUObjectHandle WorldHandle,
                       RustealUClassHandle ClsHandle, const uint8 *TransformBuf,
                       uint32 TransformSize, RustealUObjectHandle OwnerHandle,
                       RustealUObjectHandle InstigatorHandle,
                       uint8 CollisionMethod) {
  UWorld *World = Cast<UWorld>(static_cast<UObject *>(WorldHandle.ptr));
  UClass *Class = static_cast<UClass *>(ClsHandle.ptr);

  if (!World || !Class) {
    return RustealUObjectHandle{nullptr};
  }

  FTransform SpawnTransform = FTransform::Identity;

  if (TransformBuf && TransformSize > 0) {
    const uint32 CopySize =
        FMath::Min(TransformSize, static_cast<uint32>(sizeof(FTransform)));

    FMemory::Memcpy(&SpawnTransform, TransformBuf, CopySize);
  }

  FActorSpawnParameters Params;
  Params.bDeferConstruction = true;

  Params.SpawnCollisionHandlingOverride =
      static_cast<ESpawnActorCollisionHandlingMethod>(CollisionMethod);

  AActor *Owner = Cast<AActor>(static_cast<UObject *>(OwnerHandle.ptr));

  if (Owner) {
    Params.Owner = Owner;
  }

  APawn *Instigator = Cast<APawn>(static_cast<UObject *>(InstigatorHandle.ptr));

  if (Instigator) {
    Params.Instigator = Instigator;
  }

  AActor *Spawned = World->SpawnActor(Class, &SpawnTransform, Params);
  return RustealUObjectHandle{Spawned};
}

static ERustealErrorCode FinishSpawningImpl(RustealUObjectHandle ActorHandle,
                                            const uint8 *TransformBuf,
                                            uint32 TransformSize) {
  AActor *Actor = Cast<AActor>(static_cast<UObject *>(ActorHandle.ptr));

  if (!Actor)
    return RustealFail(ERustealErrorCode::NullArgument);

  FTransform SpawnTransform = FTransform::Identity;

  if (TransformBuf && TransformSize > 0) {
    const uint32 CopySize =
        FMath::Min(TransformSize, static_cast<uint32>(sizeof(FTransform)));

    FMemory::Memcpy(&SpawnTransform, TransformBuf, CopySize);
  }

  Actor->FinishSpawning(SpawnTransform);
  return ERustealErrorCode::Ok;
}

static uint8 ChannelToObjectTypeImpl(uint8 Channel) {
  return static_cast<uint8>(UEngineTypes::ConvertToObjectType(
      static_cast<ECollisionChannel>(Channel)));
}

static uint8 *FindDataTableRowImpl(RustealUObjectHandle TableHandle,
                                   RustealFNameHandle RowName,
                                   RustealUStructHandle RowStruct) {
  const UDataTable *Table =
      Cast<UDataTable>(static_cast<UObject *>(TableHandle.ptr));

  const UScriptStruct *Expected = static_cast<UScriptStruct *>(RowStruct.ptr);

  if (!Table || !Expected || !Table->GetRowStruct() ||
      !Table->GetRowStruct()->IsChildOf(Expected)) {
    return nullptr;
  }

  const FNameEntryId Index = FNameEntryId::FromUnstableInt(
      static_cast<uint32>(RowName.value & 0xFFFFFFFF));

  const FName Name(Index, Index, static_cast<int32>(RowName.value >> 32));
  return Table->FindRowUnchecked(Name);
}

ERustealErrorCode RustealTimer_SetTimer(RustealUObjectHandle Context,
                                        float Rate, bool bLooping,
                                        bool bMaxOncePerFrame, float FirstDelay,
                                        uint64 CallbackId, uint64 *OutHandle);
ERustealErrorCode RustealTimer_SetTimerForNextTick(RustealUObjectHandle Context,
                                                   uint64 CallbackId,
                                                   uint64 *OutHandle);
ERustealErrorCode RustealTimer_ClearTimer(RustealUObjectHandle Context,
                                          uint64 Packed);
ERustealErrorCode RustealTimer_PauseTimer(RustealUObjectHandle Context,
                                          uint64 Packed, bool bPaused);
float RustealTimer_TimerRemaining(RustealUObjectHandle Context, uint64 Packed);
uint32 RustealTimer_LiveTimerCallbacks();
ERustealErrorCode RustealLoading_LoadAsync(const uint8 *Paths, uint32 PathsLen,
                                           uint64 CallbackId);
ERustealErrorCode RustealLoading_LoadPrimaryAssets(const uint8 *Ids,
                                                   uint32 IdsLen,
                                                   const uint8 *Bundles,
                                                   uint32 BundlesLen,
                                                   uint64 CallbackId);
ERustealErrorCode RustealLoading_Finish(uint64 CallbackId,
                                        RustealUObjectHandle *OutObjects,
                                        uint32 Capacity, uint32 *OutCount);
ERustealErrorCode RustealLoading_Cancel(uint64 CallbackId);
uint32 RustealLoading_InProgress();
ERustealErrorCode RustealEvents_Bind(uint32 Kind, RustealUObjectHandle Target,
                                     uint64 CallbackId);
ERustealErrorCode RustealEvents_Unbind(uint64 CallbackId);
ERustealErrorCode RustealHttp_Send(const uint8 *Verb, uint32 VerbLen,
                                   const uint8 *Url, uint32 UrlLen,
                                   const uint8 *Headers, uint32 HeadersLen,
                                   const uint8 *Body, uint32 BodyLen,
                                   float TimeoutSeconds, uint64 CallbackId);
ERustealErrorCode RustealHttp_Finish(uint64 CallbackId, int32 *OutStatus,
                                     uint8 *Headers, uint32 HeadersCapacity,
                                     uint32 *OutHeadersLen, uint8 *Body,
                                     uint32 BodyCapacity, uint32 *OutBodyLen,
                                     bool *OutSucceeded);
ERustealErrorCode RustealHttp_Cancel(uint64 CallbackId);
uint32 RustealHttp_InProgress();
ERustealErrorCode RustealTick_Add(RustealUObjectHandle Target, uint8 Group,
                                  uint64 CallbackId);
void RustealTick_Remove(uint64 CallbackId);
ERustealErrorCode RustealTick_SetEnabled(uint64 CallbackId, bool bEnabled);
ERustealErrorCode RustealTick_SetInterval(uint64 CallbackId, float Seconds);
ERustealErrorCode RustealTick_AddPrerequisite(uint64 CallbackId,
                                              RustealUObjectHandle Object,
                                              uint64 OtherCallbackId);
uint32 RustealTick_LiveFunctions();
ERustealErrorCode RustealTags_AddNative(const uint8 *Name, uint32 NameLen,
                                        const uint8 *Comment,
                                        uint32 CommentLen);
bool RustealTags_IsRegistered(const uint8 *Name, uint32 NameLen);
void RustealNet_RpcValidateFailed(const uint8 *Reason, uint32 ReasonLen);
void RustealNet_MarkPropertyDirty(RustealUObjectHandle Object,
                                  RustealFPropertyHandle Prop);
ERustealErrorCode RustealNet_AddReplicatedSubobject(RustealUObjectHandle Actor,
                                                    RustealUObjectHandle Object,
                                                    uint8 Condition);
ERustealErrorCode
RustealNet_RemoveReplicatedSubobject(RustealUObjectHandle Actor,
                                     RustealUObjectHandle Object);

FRustealWorldApi GWorldApi = {
    &SpawnActorImpl,
    &GetAllActorsOfClassImpl,
    &FindObjectImpl,
    &LoadObjectImpl,
    &GetWorldImpl,
    &NewObjectImpl,
    &SpawnActorDeferredImpl,
    &FinishSpawningImpl,
    &ChannelToObjectTypeImpl,
    &FindDataTableRowImpl,
    &RustealTimer_SetTimer,
    &RustealTimer_SetTimerForNextTick,
    &RustealTimer_ClearTimer,
    &RustealTimer_PauseTimer,
    &RustealTimer_TimerRemaining,
    &RustealTimer_LiveTimerCallbacks,
    &RustealLoading_LoadAsync,
    &RustealLoading_LoadPrimaryAssets,
    &RustealLoading_Finish,
    &RustealLoading_Cancel,
    &RustealLoading_InProgress,
    &RustealEvents_Bind,
    &RustealEvents_Unbind,
    &RustealHttp_Send,
    &RustealHttp_Finish,
    &RustealHttp_Cancel,
    &RustealHttp_InProgress,
    &RustealTick_Add,
    &RustealTick_Remove,
    &RustealTick_SetEnabled,
    &RustealTick_SetInterval,
    &RustealTick_AddPrerequisite,
    &RustealTick_LiveFunctions,
    &RustealTags_AddNative,
    &RustealTags_IsRegistered,
    &RustealNet_RpcValidateFailed,
    &RustealNet_MarkPropertyDirty,
    &RustealNet_AddReplicatedSubobject,
    &RustealNet_RemoveReplicatedSubobject,
};
