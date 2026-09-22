// RustealWorldApiImpl.cpp — FRustealWorldApi implementation.

#include "RustealApiTable.h"
#include "UObject/UObjectGlobals.h"
#include "Engine/World.h"
#include "GameFramework/Actor.h"
#include "GameFramework/Pawn.h"
#include "EngineUtils.h"

// Helper: convert UTF-8 byte slice to FString.
static FString Utf8ToFStr(const uint8* Buf, uint32 Len)
{
    return FString(Len, UTF8_TO_TCHAR(reinterpret_cast<const char*>(Buf)));
}

// ---------------------------------------------------------------------------
// Implementations
// ---------------------------------------------------------------------------

static RustealUObjectHandle SpawnActorImpl(
    RustealUObjectHandle WorldHandle,
    RustealUClassHandle ClsHandle,
    const uint8* TransformBuf,
    uint32 TransformSize,
    RustealUObjectHandle OwnerHandle)
{
    UWorld* World = Cast<UWorld>(static_cast<UObject*>(WorldHandle.ptr));
    UClass* Class = static_cast<UClass*>(ClsHandle.ptr);
    if (!World || !Class)
    {
        return RustealUObjectHandle{ nullptr };
    }

    // Copy transform from Rust buffer. The buffer comes from UScriptStruct::GetStructureSize()
    // which may be smaller than sizeof(FTransform) due to C++ SIMD alignment padding.
    // Copy what we have and leave the rest as identity.
    FTransform SpawnTransform = FTransform::Identity;
    if (TransformBuf && TransformSize > 0)
    {
        const uint32 CopySize = FMath::Min(TransformSize, static_cast<uint32>(sizeof(FTransform)));
        FMemory::Memcpy(&SpawnTransform, TransformBuf, CopySize);
    }

    FActorSpawnParameters Params;
    AActor* Owner = Cast<AActor>(static_cast<UObject*>(OwnerHandle.ptr));
    if (Owner)
    {
        Params.Owner = Owner;
    }

    AActor* Spawned = World->SpawnActor(Class, &SpawnTransform, Params);
    return RustealUObjectHandle{ Spawned };
}

static ERustealErrorCode GetAllActorsOfClassImpl(
    RustealUObjectHandle WorldHandle,
    RustealUClassHandle ClsHandle,
    uint8* OutBuf,
    uint32 BufByteSize,
    uint32* OutCount)
{
    UWorld* World = Cast<UWorld>(static_cast<UObject*>(WorldHandle.ptr));
    UClass* Class = static_cast<UClass*>(ClsHandle.ptr);
    if (!World || !Class)
    {
        if (OutCount) *OutCount = 0;
        return ERustealErrorCode::NullArgument;
    }

    const uint32 BufCapacity = BufByteSize / static_cast<uint32>(sizeof(RustealUObjectHandle));
    RustealUObjectHandle* HandleBuf = reinterpret_cast<RustealUObjectHandle*>(OutBuf);
    uint32 Count = 0;
    for (TActorIterator<AActor> It(World, Class); It; ++It)
    {
        if (HandleBuf && Count < BufCapacity)
        {
            HandleBuf[Count] = RustealUObjectHandle{ *It };
        }
        ++Count;
    }

    if (OutCount) *OutCount = Count;
    return ERustealErrorCode::Ok;
}

static RustealUObjectHandle FindObjectImpl(
    RustealUClassHandle ClsHandle,
    const uint8* PathUtf8,
    uint32 PathLen)
{
    UClass* Class = static_cast<UClass*>(ClsHandle.ptr);
    const FString Path = Utf8ToFStr(PathUtf8, PathLen);

    UObject* Found = StaticFindObject(Class, nullptr, *Path);
    return RustealUObjectHandle{ Found };
}

static RustealUObjectHandle LoadObjectImpl(
    RustealUClassHandle ClsHandle,
    const uint8* PathUtf8,
    uint32 PathLen)
{
    UClass* Class = static_cast<UClass*>(ClsHandle.ptr);
    if (!Class) Class = UObject::StaticClass();
    const FString Path = Utf8ToFStr(PathUtf8, PathLen);

    UObject* Loaded = StaticLoadObject(Class, nullptr, *Path);
    return RustealUObjectHandle{ Loaded };
}

static RustealUObjectHandle GetWorldImpl(RustealUObjectHandle ActorHandle)
{
    AActor* Actor = Cast<AActor>(static_cast<UObject*>(ActorHandle.ptr));
    if (!Actor) return RustealUObjectHandle{ nullptr };
    UWorld* World = Actor->GetWorld();
    return RustealUObjectHandle{ World };
}

static RustealUObjectHandle NewObjectImpl(RustealUObjectHandle OuterHandle, RustealUClassHandle ClsHandle)
{
    UClass* Class = static_cast<UClass*>(ClsHandle.ptr);
    if (!Class) return RustealUObjectHandle{ nullptr };
    UObject* Outer = static_cast<UObject*>(OuterHandle.ptr);
    if (!Outer) Outer = GetTransientPackage();
    UObject* Obj = NewObject<UObject>(Outer, Class);
    return RustealUObjectHandle{ Obj };
}

static RustealUObjectHandle SpawnActorDeferredImpl(
    RustealUObjectHandle WorldHandle,
    RustealUClassHandle ClsHandle,
    const uint8* TransformBuf,
    uint32 TransformSize,
    RustealUObjectHandle OwnerHandle,
    RustealUObjectHandle InstigatorHandle,
    uint8 CollisionMethod)
{
    UWorld* World = Cast<UWorld>(static_cast<UObject*>(WorldHandle.ptr));
    UClass* Class = static_cast<UClass*>(ClsHandle.ptr);
    if (!World || !Class)
    {
        return RustealUObjectHandle{ nullptr };
    }

    FTransform SpawnTransform = FTransform::Identity;
    if (TransformBuf && TransformSize > 0)
    {
        const uint32 CopySize = FMath::Min(TransformSize, static_cast<uint32>(sizeof(FTransform)));
        FMemory::Memcpy(&SpawnTransform, TransformBuf, CopySize);
    }

    FActorSpawnParameters Params;
    Params.bDeferConstruction = true;
    Params.SpawnCollisionHandlingOverride =
        static_cast<ESpawnActorCollisionHandlingMethod>(CollisionMethod);

    AActor* Owner = Cast<AActor>(static_cast<UObject*>(OwnerHandle.ptr));
    if (Owner)
    {
        Params.Owner = Owner;
    }

    APawn* Instigator = Cast<APawn>(static_cast<UObject*>(InstigatorHandle.ptr));
    if (Instigator)
    {
        Params.Instigator = Instigator;
    }

    AActor* Spawned = World->SpawnActor(Class, &SpawnTransform, Params);
    return RustealUObjectHandle{ Spawned };
}

static ERustealErrorCode FinishSpawningImpl(
    RustealUObjectHandle ActorHandle,
    const uint8* TransformBuf,
    uint32 TransformSize)
{
    AActor* Actor = Cast<AActor>(static_cast<UObject*>(ActorHandle.ptr));
    if (!Actor) return ERustealErrorCode::NullArgument;

    FTransform SpawnTransform = FTransform::Identity;
    if (TransformBuf && TransformSize > 0)
    {
        const uint32 CopySize = FMath::Min(TransformSize, static_cast<uint32>(sizeof(FTransform)));
        FMemory::Memcpy(&SpawnTransform, TransformBuf, CopySize);
    }

    Actor->FinishSpawning(SpawnTransform);
    return ERustealErrorCode::Ok;
}

// ---------------------------------------------------------------------------
// Static instance
// ---------------------------------------------------------------------------

FRustealWorldApi GWorldApi = {
    &SpawnActorImpl,
    &GetAllActorsOfClassImpl,
    &FindObjectImpl,
    &LoadObjectImpl,
    &GetWorldImpl,
    &NewObjectImpl,
    &SpawnActorDeferredImpl,
    &FinishSpawningImpl,
};
