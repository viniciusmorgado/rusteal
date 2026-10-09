#include "RustealApiTable.h"
#include "RustealLibrary.h"
#include "RustealModule.h"
#include "UObject/UObjectArray.h"
#include "UObject/UObjectGlobals.h"

static void AddGcRootImpl(RustealUObjectHandle Obj) {
  UObject *Object = static_cast<UObject *>(Obj.ptr);

  if (::IsValid(Object)) {
    Object->AddToRoot();
  }
}

static void RemoveGcRootImpl(RustealUObjectHandle Obj) {
  UObject *Object = static_cast<UObject *>(Obj.ptr);

  if (::IsValid(Object)) {
    Object->RemoveFromRoot();
  }
}

static TMap<const UObjectBase *, TArray<FRustealLibrary *, TInlineAllocator<1>>>
    GPinnedObjects;

class FRustealPinnedDeleteListener
    : public FUObjectArray::FUObjectDeleteListener {
public:
  virtual void NotifyUObjectDeleted(const UObjectBase *Object,
                                    int32 Index) override {
    TArray<FRustealLibrary *, TInlineAllocator<1>> Libraries;

    if (!GPinnedObjects.RemoveAndCopyValue(Object, Libraries)) {
      return;
    }

    for (FRustealLibrary *Library : Libraries) {
      RustealCallLibrary(Library, [Object](const FRustealRustCallbacks &Cb) {
        Cb.notify_pinned_destroyed(
            RustealUObjectHandle{const_cast<UObjectBase *>(Object)});
      });
    }
  }

  virtual void OnUObjectArrayShutdown() override {
    GUObjectArray.RemoveUObjectDeleteListener(this);
  }
};

static FRustealPinnedDeleteListener GPinnedDeleteListener;
static bool GPinnedListenerRegistered = false;

static void EnsurePinnedListenerRegistered() {
  if (!GPinnedListenerRegistered) {
    GUObjectArray.AddUObjectDeleteListener(&GPinnedDeleteListener);
    GPinnedListenerRegistered = true;
  }
}

static void RegisterPinnedImpl(RustealUObjectHandle Obj) {
  const UObjectBase *Object = static_cast<const UObjectBase *>(Obj.ptr);

  if (Object) {
    EnsurePinnedListenerRegistered();
    GPinnedObjects.FindOrAdd(Object).AddUnique(RustealCurrentLibrary());
  }
}

static void UnregisterPinnedImpl(RustealUObjectHandle Obj) {
  const UObjectBase *Object = static_cast<const UObjectBase *>(Obj.ptr);

  if (auto *Libraries = GPinnedObjects.Find(Object)) {
    Libraries->Remove(RustealCurrentLibrary());

    if (Libraries->IsEmpty()) {
      GPinnedObjects.Remove(Object);
    }
  }
}

static void UnrootPinned(const UObjectBase *TrackedBase) {
  UObject *Object =
      static_cast<UObject *>(const_cast<UObjectBase *>(TrackedBase));

  if (::IsValid(Object) && Object->IsRooted()) {
    Object->RemoveFromRoot();
  }
}

void RustealPinnedForgetLibrary(FRustealLibrary *Library) {
  for (auto It = GPinnedObjects.CreateIterator(); It; ++It) {
    It.Value().Remove(Library);

    if (It.Value().IsEmpty()) {
      UnrootPinned(It.Key());
      It.RemoveCurrent();
    }
  }
}

void RustealPinnedShutdown() {
  if (GPinnedListenerRegistered) {
    GUObjectArray.RemoveUObjectDeleteListener(&GPinnedDeleteListener);
    GPinnedListenerRegistered = false;
  }

  for (const auto &Pair : GPinnedObjects) {
    UnrootPinned(Pair.Key);
  }

  GPinnedObjects.Empty();
}

FRustealLifecycleApi GLifecycleApi = {
    &AddGcRootImpl,
    &RemoveGcRootImpl,
    &RegisterPinnedImpl,
    &UnregisterPinnedImpl,
};
