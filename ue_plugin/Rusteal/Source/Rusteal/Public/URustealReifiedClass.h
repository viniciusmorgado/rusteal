#pragma once

#include <atomic>

#include "URustealReifiedClass.generated.h"

struct FRustealLibrary;

struct FRustealComponentDef {
  FName SubobjectName;
  FName PropertyName;
  UClass *ComponentClass = nullptr;
  bool bIsRoot = false;
  bool bIsTransient = false;
  FName AttachParentName;
  FName AttachSocketName;
};

UCLASS()
class URustealReifiedClass : public UBlueprintGeneratedClass {
  GENERATED_BODY()

public:
  uint64 RustTypeId = 0;

  FRustealLibrary *Library = nullptr;

  uint64 Shape = 0;

  UPROPERTY()
  TObjectPtr<UClass> NativeSuperClass = nullptr;

  TArray<FRustealComponentDef> ComponentDefs;

  static void
  RustealClassConstructor(const FObjectInitializer &ObjectInitializer);

  static TArray<URustealReifiedClass *> ReifiedChain(const UClass *Class);

  virtual UClass *GetAuthoritativeClass() override;

  virtual void
  InitPropertiesFromCustomList(uint8 *DataPtr,
                               const uint8 *DefaultDataPtr) override;

  void InvalidateCustomPropertyList();

private:
  std::atomic<bool> bCustomPropertyListCurrent{false};
  FCriticalSection CustomPropertyListLock;
};
