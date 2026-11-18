#include "Misc/ScopeLock.h"
#include "RustealApiTable.h"
#include "RustealLibrary.h"
#include "RustealModule.h"
#include "UObject/Class.h"
#include "UObject/GCObject.h"

// What Rust keeps alive: objects pinned with Pinned<T>, and the engine structs Rust owns
// (OwnedStruct<T>), whose object members would otherwise be invisible to the garbage collector.
// Both are reported as references, the way a UPROPERTY member is; nothing is added to the root
// set, so an object the engine roots itself is never unrooted by Rust.
//
// Each pin is a slot of its own: two Pinned of one object keep it twice, and dropping one leaves
// the other. A pinned object destroyed explicitly (an actor's Destroy) is still collected; Rust
// sees it gone through the index and serial number it keeps.
//
// Structs can be tracked from a worker thread (a #[ufunction(ThreadSafe)] building one), so the
// lists are guarded.
class FRustealReferenceCollector final : public FGCObject {
public:
  uint32 Pin(UObject *Object) {
    FScopeLock Lock(&Guard);
    const FSlot Slot{Object, RustealCurrentLibrary(), true};

    if (Free.Num() > 0) {
      const int32 Index = Free.Pop(EAllowShrinking::No);
      Slots[Index] = Slot;
      return static_cast<uint32>(Index) + 1;
    }

    Slots.Add(Slot);
    return static_cast<uint32>(Slots.Num());
  }

  void Unpin(uint32 Id) {
    FScopeLock Lock(&Guard);
    const int32 Index = static_cast<int32>(Id) - 1;

    if (!Slots.IsValidIndex(Index) || !Slots[Index].bUsed) {
      return;
    }

    Slots[Index] = FSlot{};
    Free.Add(Index);
  }

  void TrackStruct(const UScriptStruct *Struct, uint8 *Memory) {
    // A struct with no object member has nothing to report.
    if (!Struct || !Memory ||
        (!Struct->RefLink &&
         !(Struct->StructFlags & STRUCT_AddStructReferencedObjects))) {
      return;
    }

    FScopeLock Lock(&Guard);
    Structs.Add(Memory, FTrackedStruct{Struct, RustealCurrentLibrary()});
  }

  void UntrackStruct(uint8 *Memory) {
    FScopeLock Lock(&Guard);
    Structs.Remove(Memory);
  }

  void ForgetLibrary(FRustealLibrary *Library) {
    FScopeLock Lock(&Guard);

    for (int32 Index = 0; Index < Slots.Num(); ++Index) {
      if (Slots[Index].bUsed && Slots[Index].Library == Library) {
        Slots[Index] = FSlot{};
        Free.Add(Index);
      }
    }

    for (auto It = Structs.CreateIterator(); It; ++It) {
      if (It.Value().Library == Library) {
        It.RemoveCurrent();
      }
    }
  }

  virtual void AddReferencedObjects(FReferenceCollector &Collector) override {
    FScopeLock Lock(&Guard);

    for (FSlot &Slot : Slots) {
      if (Slot.bUsed) {
        Collector.AddReferencedObject(Slot.Object);
      }
    }

    for (auto &Pair : Structs) {
      Collector.AddPropertyReferencesWithStructARO(Pair.Value.Struct,
                                                   Pair.Key);
    }
  }

  virtual FString GetReferencerName() const override {
    return TEXT("FRustealReferenceCollector");
  }

private:
  struct FSlot {
    TObjectPtr<UObject> Object = nullptr;
    FRustealLibrary *Library = nullptr;
    bool bUsed = false;
  };

  struct FTrackedStruct {
    const UScriptStruct *Struct = nullptr;
    FRustealLibrary *Library = nullptr;
  };

  FCriticalSection Guard;
  TArray<FSlot> Slots;
  TArray<int32> Free;
  TMap<void *, FTrackedStruct> Structs;
};

// Made on first use: an FGCObject registers with the garbage collector when it is constructed,
// which a static would do before the object system is up.
static TUniquePtr<FRustealReferenceCollector> GReferenceCollector;
static FCriticalSection GReferenceCollectorGuard;

static FRustealReferenceCollector &ReferenceCollector() {
  FScopeLock Lock(&GReferenceCollectorGuard);

  if (!GReferenceCollector) {
    GReferenceCollector = MakeUnique<FRustealReferenceCollector>();
  }

  return *GReferenceCollector;
}

static uint32 PinImpl(RustealUObjectHandle Obj) {
  UObject *Object = static_cast<UObject *>(Obj.ptr);

  if (!::IsValid(Object)) {
    return 0;
  }

  return ReferenceCollector().Pin(Object);
}

static void UnpinImpl(uint32 Slot) {
  if (Slot != 0 && GReferenceCollector) {
    GReferenceCollector->Unpin(Slot);
  }
}

static void TrackStructImpl(RustealUStructHandle Struct, uint8 *Memory) {
  ReferenceCollector().TrackStruct(static_cast<const UScriptStruct *>(Struct.ptr),
                                   Memory);
}

static void UntrackStructImpl(uint8 *Memory) {
  if (GReferenceCollector) {
    GReferenceCollector->UntrackStruct(Memory);
  }
}

void RustealPinnedForgetLibrary(FRustealLibrary *Library) {
  if (GReferenceCollector) {
    GReferenceCollector->ForgetLibrary(Library);
  }
}

void RustealPinnedShutdown() {
  FScopeLock Lock(&GReferenceCollectorGuard);
  GReferenceCollector.Reset();
}

FRustealLifecycleApi GLifecycleApi = {
    &PinImpl,
    &UnpinImpl,
    &TrackStructImpl,
    &UntrackStructImpl,
};
