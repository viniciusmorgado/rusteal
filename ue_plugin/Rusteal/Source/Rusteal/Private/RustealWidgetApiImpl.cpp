// RustealWidgetApiImpl.cpp — FRustealWidgetApi implementation.
// Provides CreateWidget (C++ template, not in reflection), WidgetTree, and RootWidget access.

#include "RustealApiTable.h"
#include "Blueprint/UserWidget.h"
#include "Blueprint/WidgetTree.h"
#include "Components/Widget.h"
#include "GameFramework/PlayerController.h"
#include "Engine/GameInstance.h"
#include "Engine/World.h"

// ---------------------------------------------------------------------------
// Implementations
// ---------------------------------------------------------------------------

static RustealUObjectHandle CreateWidgetImpl(
    RustealUObjectHandle OwningHandle,
    RustealUClassHandle WidgetClassHandle)
{
    UClass* WidgetClass = static_cast<UClass*>(WidgetClassHandle.ptr);
    UObject* Owning = static_cast<UObject*>(OwningHandle.ptr);
    if (!WidgetClass || !Owning)
    {
        return RustealUObjectHandle{ nullptr };
    }

    // Try casting to supported owning types in order of likelihood.
    if (APlayerController* PC = Cast<APlayerController>(Owning))
    {
        UUserWidget* Widget = CreateWidget<UUserWidget>(PC, WidgetClass);
        return RustealUObjectHandle{ Widget };
    }
    if (UWorld* World = Cast<UWorld>(Owning))
    {
        UUserWidget* Widget = CreateWidget<UUserWidget>(World, WidgetClass);
        return RustealUObjectHandle{ Widget };
    }
    if (UGameInstance* GI = Cast<UGameInstance>(Owning))
    {
        UUserWidget* Widget = CreateWidget<UUserWidget>(GI, WidgetClass);
        return RustealUObjectHandle{ Widget };
    }

    // Fallback: try GetWorld() on the owning object.
    UWorld* World = Owning->GetWorld();
    if (World)
    {
        UUserWidget* Widget = CreateWidget<UUserWidget>(World, WidgetClass);
        return RustealUObjectHandle{ Widget };
    }

    return RustealUObjectHandle{ nullptr };
}

static ERustealErrorCode SetRootWidgetImpl(
    RustealUObjectHandle UserWidgetHandle,
    RustealUObjectHandle RootWidgetHandle)
{
    UUserWidget* UserWidget = Cast<UUserWidget>(static_cast<UObject*>(UserWidgetHandle.ptr));
    UWidget* RootWidget = Cast<UWidget>(static_cast<UObject*>(RootWidgetHandle.ptr));
    if (!UserWidget) return ERustealErrorCode::NullArgument;
    if (!RootWidget) return ERustealErrorCode::NullArgument;

    UWidgetTree* Tree = UserWidget->WidgetTree;
    if (!Tree) return ERustealErrorCode::InvalidOperation;

    Tree->RootWidget = RootWidget;
    return ERustealErrorCode::Ok;
}

static RustealUObjectHandle GetWidgetTreeImpl(RustealUObjectHandle UserWidgetHandle)
{
    UUserWidget* UserWidget = Cast<UUserWidget>(static_cast<UObject*>(UserWidgetHandle.ptr));
    if (!UserWidget) return RustealUObjectHandle{ nullptr };
    return RustealUObjectHandle{ UserWidget->WidgetTree };
}

// ---------------------------------------------------------------------------
// Static instance
// ---------------------------------------------------------------------------

FRustealWidgetApi GWidgetApi = {
    &CreateWidgetImpl,
    &SetRootWidgetImpl,
    &GetWidgetTreeImpl,
};
