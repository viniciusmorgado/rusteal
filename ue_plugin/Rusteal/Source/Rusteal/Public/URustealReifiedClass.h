#pragma once

#include <atomic>

#include "URustealReifiedClass.generated.h"

// Describes a default subobject to be created during class construction.
struct FRustealComponentDef {
  FName SubobjectName;
  FName PropertyName; // the property referencing it; often SubobjectName
  UClass *ComponentClass = nullptr;
  bool bIsRoot = false;
  bool bIsTransient = false;
  FName AttachParentName; // NAME_None = no parent
  FName AttachSocketName; // NAME_None = no socket
};

// A UClass created at runtime by Rust via the Reify API.
// Inherits from UBlueprintGeneratedClass so the engine treats it
// similarly to Blueprint classes (CDO creation, property editing, etc.).
UCLASS()
class URustealReifiedClass : public UBlueprintGeneratedClass {
  GENERATED_BODY()

public:
  // Rust type ID — used to look up the correct Rust type info
  // (constructor, destructor) in the Rust-side registry.
  uint64 RustTypeId = 0;

  // The native (C++) superclass. For a Rust class inheriting AActor,
  // this would be AActor::StaticClass(). Used to call the correct
  // native constructor.
  UPROPERTY()
  TObjectPtr<UClass> NativeSuperClass = nullptr;

  // Default subobject definitions registered from Rust.
  TArray<FRustealComponentDef> ComponentDefs;

  // Custom constructor called by UE when instantiating objects of this class.
  static void
  RustealClassConstructor(const FObjectInitializer &ObjectInitializer);

  // The Rust classes among Class and its supers, the topmost first: a Rust
  // class whose parent is a Rust class, or a Blueprint child of one.
  static TArray<URustealReifiedClass *> ReifiedChain(const UClass *Class);

  // Override: UBlueprintGeneratedClass assumes ClassGeneratedBy points to a
  // UBlueprint asset.  Reified classes have no Blueprint, so return this
  // directly.
  virtual UClass *GetAuthoritativeClass() override;

  // Override: an instance gets the values the class default object holds for
  // properties of native classes (what #[class_defaults] sets) from the
  // custom property list, which the native constructor would otherwise
  // leave at the native defaults. Rust writes the class defaults after the
  // class is finalized, so the list is built on first use.
  virtual void InitPropertiesFromCustomList(uint8 *DataPtr,
                                            const uint8 *DefaultDataPtr) override;

  // The class defaults may have changed (class finalized or reloaded): build
  // the custom property list again before the next instance.
  void InvalidateCustomPropertyList();

private:
  std::atomic<bool> bCustomPropertyListCurrent{false};
  FCriticalSection CustomPropertyListLock;
};
