#include "URustealReifiedClass.h"
#include "RustealApiTable.h"
#include "RustealModule.h"
#include "Components/SceneComponent.h"
#include "GameFramework/Actor.h"

UClass* URustealReifiedClass::GetAuthoritativeClass()
{
    // Reified classes have no UBlueprint asset, so this class IS the
    // authoritative class.  The base implementation would crash on the
    // null ClassGeneratedBy pointer.
    return this;
}

void URustealReifiedClass::RustealClassConstructor(const FObjectInitializer& ObjectInitializer)
{
    // 1. Find the URustealReifiedClass in the hierarchy. The immediate class may be
    //    a Blueprint child (e.g. SKEL_new_MacroTestActor_C), so walk up.
    URustealReifiedClass* ReifiedClass = nullptr;
    for (UClass* Cls = ObjectInitializer.GetClass(); Cls; Cls = Cls->GetSuperClass())
    {
        ReifiedClass = Cast<URustealReifiedClass>(Cls);
        if (ReifiedClass) break;
    }
    if (!ReifiedClass)
    {
        UE_LOG(LogRusteal, Error, TEXT("[Rusteal] RustealClassConstructor called on non-reified class!"));
        return;
    }

    // 2. Call the native super's constructor to initialize UE-side state.
    UClass* NativeSuper = ReifiedClass->NativeSuperClass;
    if (NativeSuper && NativeSuper->ClassConstructor)
    {
        NativeSuper->ClassConstructor(ObjectInitializer);
    }

    // 3. Create default subobjects from Rust-registered definitions.
    UObject* Obj = ObjectInitializer.GetObj();
    if (ReifiedClass->ComponentDefs.Num() > 0)
    {
        TMap<FName, USceneComponent*> CreatedComponents;

        for (const FRustealComponentDef& Def : ReifiedClass->ComponentDefs)
        {
            UObject* Sub = ObjectInitializer.CreateDefaultSubobject(
                Obj, Def.SubobjectName,
                Def.ComponentClass, Def.ComponentClass,
                /*bIsRequired=*/true, Def.bIsTransient);

            if (!Sub) continue;

            USceneComponent* SceneComp = Cast<USceneComponent>(Sub);
            if (SceneComp)
            {
                CreatedComponents.Add(Def.SubobjectName, SceneComp);
            }

            if (Def.bIsRoot)
            {
                if (AActor* Actor = Cast<AActor>(Obj))
                {
                    if (SceneComp) Actor->SetRootComponent(SceneComp);
                }
            }
            else if (Def.AttachParentName != NAME_None)
            {
                if (SceneComp)
                {
                    if (USceneComponent** Parent = CreatedComponents.Find(Def.AttachParentName))
                    {
                        SceneComp->SetupAttachment(*Parent);
                    }
                }
            }
        }
    }

    // 4. Notify Rust to construct its instance data.
    const FRustealRustCallbacks* Callbacks = GetRustealRustCallbacks();
    if (Callbacks && Callbacks->construct_rust_instance)
    {
        bool bIsCDO = Obj->HasAnyFlags(RF_ClassDefaultObject);
        Callbacks->construct_rust_instance(
            RustealUObjectHandle{ Obj },
            ReifiedClass->RustTypeId,
            bIsCDO);
    }
}
