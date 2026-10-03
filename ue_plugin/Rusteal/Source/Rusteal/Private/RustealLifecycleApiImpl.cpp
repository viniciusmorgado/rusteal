// RustealLifecycleApiImpl.cpp — FRustealLifecycleApi implementation.
//
// Provides GC root management and Pinned object destroy notification.
// - add_gc_root / remove_gc_root: prevent/allow UE garbage collection
// - register_pinned / unregister_pinned: track Pinned objects for destroy
// notification

#include "RustealApiTable.h"
#include "RustealLibrary.h"
#include "RustealModule.h"
#include "UObject/UObjectArray.h"
#include "UObject/UObjectGlobals.h"

// ---------------------------------------------------------------------------
// GC root management
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Pinned object tracking + destroy notification
// ---------------------------------------------------------------------------

// The objects Rust holds Pinned<T> handles to, each with the libraries that
// pinned it. Checked by the delete listener to fire notify_pinned_destroyed in
// those libraries.
static TMap<const UObjectBase *, TArray<FRustealLibrary *, TInlineAllocator<1>>>
    GPinnedObjects;

// Delete listener that watches GUObjectArray for pinned object destruction.
// Extends the existing FRustealDeleteListener pattern from
// RustealReifyApiImpl.cpp.
class FRustealPinnedDeleteListener
    : public FUObjectArray::FUObjectDeleteListener {
public:
  virtual void NotifyUObjectDeleted(const UObjectBase *Object,
                                    int32 Index) override {
    // Remove from tracking first — the Pinned<T> drop will call
    // unregister_pinned but the object is already gone, so we clean up
    // proactively.
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

// Remove an object from the root set its Pinned<T> handles put it in.
static void UnrootPinned(const UObjectBase *TrackedBase) {
  UObject *Object =
      static_cast<UObject *>(const_cast<UObjectBase *>(TrackedBase));
  if (::IsValid(Object) && Object->IsRooted()) {
    Object->RemoveFromRoot();
  }
}

// Called from RustealModule.cpp when a library unloads.
//
// Hot reload note: when a Rust library is unloaded, user statics holding
// Pinned<T> values are forgotten without their Drop running, so Rust never
// calls remove_gc_root for them. Without this loop those objects stay in
// UE's root set forever and trip the !IsRooted() assertion when PIE later
// tries to clean up the world. We mirror what Pinned<T>::drop would have
// done on the C++ side, for the objects no other library still pins.
void RustealPinnedForgetLibrary(FRustealLibrary *Library) {
  for (auto It = GPinnedObjects.CreateIterator(); It; ++It) {
    It.Value().Remove(Library);
    if (It.Value().IsEmpty()) {
      UnrootPinned(It.Key());
      It.RemoveCurrent();
    }
  }
}

// Called from RustealModule.cpp when the module shuts down.
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

// ---------------------------------------------------------------------------
// Static instance
// ---------------------------------------------------------------------------

FRustealLifecycleApi GLifecycleApi = {
    &AddGcRootImpl,
    &RemoveGcRootImpl,
    &RegisterPinnedImpl,
    &UnregisterPinnedImpl,
};
