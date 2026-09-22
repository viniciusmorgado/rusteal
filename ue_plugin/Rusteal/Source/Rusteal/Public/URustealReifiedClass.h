#pragma once

#include "CoreMinimal.h"
#include "Engine/BlueprintGeneratedClass.h"
#include "URustealReifiedClass.generated.h"

// Describes a default subobject to be created during class construction.
struct FRustealComponentDef
{
    FName SubobjectName;
    UClass* ComponentClass = nullptr;
    bool bIsRoot = false;
    bool bIsTransient = false;
    FName AttachParentName; // NAME_None = no parent
};

// A UClass created at runtime by Rust via the Reify API.
// Inherits from UBlueprintGeneratedClass so the engine treats it
// similarly to Blueprint classes (CDO creation, property editing, etc.).
UCLASS()
class URustealReifiedClass : public UBlueprintGeneratedClass
{
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
    static void RustealClassConstructor(const FObjectInitializer& ObjectInitializer);

    // Override: UBlueprintGeneratedClass assumes ClassGeneratedBy points to a
    // UBlueprint asset.  Reified classes have no Blueprint, so return this directly.
    virtual UClass* GetAuthoritativeClass() override;
};
