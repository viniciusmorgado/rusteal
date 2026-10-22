#include "Blueprint/UserWidget.h"
#include "RustealErrors.h"
#include "Blueprint/WidgetTree.h"
#include "Components/Widget.h"
#include "Engine/GameInstance.h"
#include "Engine/World.h"
#include "GameFramework/PlayerController.h"
#include "RustealApiTable.h"

static RustealUObjectHandle
CreateWidgetImpl(RustealUObjectHandle OwningHandle,
                 RustealUClassHandle WidgetClassHandle) {
  UClass *WidgetClass = static_cast<UClass *>(WidgetClassHandle.ptr);
  UObject *Owning = static_cast<UObject *>(OwningHandle.ptr);

  if (!WidgetClass || !Owning) {
    return RustealUObjectHandle{nullptr};
  }

  if (APlayerController *PC = Cast<APlayerController>(Owning)) {
    UUserWidget *Widget = CreateWidget<UUserWidget>(PC, WidgetClass);
    return RustealUObjectHandle{Widget};
  }

  if (UWorld *World = Cast<UWorld>(Owning)) {
    UUserWidget *Widget = CreateWidget<UUserWidget>(World, WidgetClass);
    return RustealUObjectHandle{Widget};
  }

  if (UGameInstance *GI = Cast<UGameInstance>(Owning)) {
    UUserWidget *Widget = CreateWidget<UUserWidget>(GI, WidgetClass);
    return RustealUObjectHandle{Widget};
  }

  UWorld *World = Owning->GetWorld();

  if (World) {
    UUserWidget *Widget = CreateWidget<UUserWidget>(World, WidgetClass);
    return RustealUObjectHandle{Widget};
  }

  return RustealUObjectHandle{nullptr};
}

static ERustealErrorCode
SetRootWidgetImpl(RustealUObjectHandle UserWidgetHandle,
                  RustealUObjectHandle RootWidgetHandle) {
  UUserWidget *UserWidget =
      Cast<UUserWidget>(static_cast<UObject *>(UserWidgetHandle.ptr));

  UWidget *RootWidget =
      Cast<UWidget>(static_cast<UObject *>(RootWidgetHandle.ptr));

  if (!UserWidget)
    return RustealFail(ERustealErrorCode::NullArgument);

  if (!RootWidget)
    return RustealFail(ERustealErrorCode::NullArgument);

  UWidgetTree *Tree = UserWidget->WidgetTree;

  if (!Tree)
    return RustealFail(ERustealErrorCode::InvalidOperation);

  Tree->RootWidget = RootWidget;
  return ERustealErrorCode::Ok;
}

static RustealUObjectHandle
GetWidgetTreeImpl(RustealUObjectHandle UserWidgetHandle) {
  UUserWidget *UserWidget =
      Cast<UUserWidget>(static_cast<UObject *>(UserWidgetHandle.ptr));

  if (!UserWidget)
    return RustealUObjectHandle{nullptr};

  return RustealUObjectHandle{UserWidget->WidgetTree};
}

uint64 RustealSlate_New(uint32 Kind, const RustealSlateProps *Props);
ERustealErrorCode RustealSlate_AddChild(uint64 Parent, uint64 Child,
                                        const RustealSlateSlot *Slot);
void RustealSlate_Release(uint64 Id);
void RustealSlate_WriteText(uint8 *Out, const uint8 *Utf8, uint32 Len);
bool RustealSlate_SimulateClick(uint64 Id);
ERustealErrorCode RustealSlate_GetText(uint64 Id, uint8 *Buffer,
                                       uint32 Capacity, uint32 *OutLen);
ERustealErrorCode RustealSlate_RegisterTab(const uint8 *Name, uint32 NameLen,
                                           const uint8 *Label, uint32 LabelLen,
                                           uint64 CallbackId);
bool RustealSlate_InvokeTab(const uint8 *Name, uint32 NameLen);
void RustealSlate_UnregisterTab(const uint8 *Name, uint32 NameLen);
uint32 RustealSlate_LiveWidgets();
uint32 RustealSlate_LiveCallbacks();
ERustealErrorCode RustealDetails_CustomizeClass(RustealUClassHandle Class,
                                                uint64 CallbackId);
void RustealDetails_UnregisterClass(RustealUClassHandle Class);
ERustealErrorCode RustealDetails_AddRow(uint64 Layout, const uint8 *Category,
                                        uint32 CategoryLen, uint64 NameWidget,
                                        uint64 ValueWidget);
ERustealErrorCode RustealDetails_HideProperty(uint64 Layout,
                                              const uint8 *Property,
                                              uint32 PropertyLen);
uint32 RustealDetails_Objects(uint64 Layout, RustealUObjectHandle *Out,
                              uint32 Capacity);
uint64 RustealDetails_View(RustealUObjectHandle Object);

FRustealWidgetApi GWidgetApi = {
    &CreateWidgetImpl,
    &SetRootWidgetImpl,
    &GetWidgetTreeImpl,
    &RustealSlate_New,
    &RustealSlate_AddChild,
    &RustealSlate_Release,
    &RustealSlate_WriteText,
    &RustealSlate_SimulateClick,
    &RustealSlate_GetText,
    &RustealSlate_RegisterTab,
    &RustealSlate_InvokeTab,
    &RustealSlate_UnregisterTab,
    &RustealSlate_LiveWidgets,
    &RustealSlate_LiveCallbacks,
    &RustealDetails_CustomizeClass,
    &RustealDetails_UnregisterClass,
    &RustealDetails_AddRow,
    &RustealDetails_HideProperty,
    &RustealDetails_Objects,
    &RustealDetails_View,
};
