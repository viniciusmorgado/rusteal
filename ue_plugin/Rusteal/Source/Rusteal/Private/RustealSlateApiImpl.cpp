#include "Framework/Application/SlateApplication.h"
#include "Framework/Docking/TabManager.h"
#include "RustealApiTable.h"
#include "RustealErrors.h"
#include "RustealLibrary.h"
#include "RustealStrings.h"
#include "Widgets/Docking/SDockTab.h"
#include "Widgets/Input/SButton.h"
#include "Widgets/Input/SCheckBox.h"
#include "Widgets/Input/SEditableTextBox.h"
#include "Widgets/Input/SSpinBox.h"
#include "Widgets/Layout/SBorder.h"
#include "Widgets/Layout/SScrollBox.h"
#include "Widgets/Layout/SSpacer.h"
#include "Widgets/SBoxPanel.h"
#include "Widgets/Text/STextBlock.h"

enum class ERustealSlateKind : uint32 {
  TextBlock = 1,
  Button = 2,
  CheckBox = 3,
  EditableTextBox = 4,
  SpinBox = 5,
  VerticalBox = 6,
  HorizontalBox = 7,
  ScrollBox = 8,
  Border = 9,
  Spacer = 10,
  Adopted = 100,
};

struct FRustealSlateCallback {
  FRustealLibrary *Library = nullptr;
  uint64 CallbackId = 0;

  ~FRustealSlateCallback();

  void Invoke(uint8 *Params) const {
    RustealCallLibrary(Library,
                       [this, Params](const FRustealRustCallbacks &Cb) {
                         Cb.invoke_delegate_callback(CallbackId, Params);
                       });
  }
};

struct FRustealSlateWidget {
  TSharedRef<SWidget> Widget;
  ERustealSlateKind Kind;
  FRustealLibrary *Library;
};

struct FRustealSlateTextArg {
  const uint8 *Utf8;
  uint32 Len;
};

static TSet<FRustealSlateCallback *> GSlateCallbacks;
static TMap<uint64, FRustealSlateWidget> GSlateWidgets;
static TMap<FName, TSharedRef<FRustealSlateCallback>> GSlateTabs;
static uint64 GNextWidgetId = 1;

FRustealSlateCallback::~FRustealSlateCallback() {
  GSlateCallbacks.Remove(this);

  RustealCallLibrary(Library, [this](const FRustealRustCallbacks &Cb) {
    Cb.release_delegate_callback(CallbackId);
  });
}

static TSharedPtr<FRustealSlateCallback> MakeCallback(uint64 CallbackId) {
  if (CallbackId == 0) {
    return nullptr;
  }

  TSharedRef<FRustealSlateCallback> Callback =
      MakeShared<FRustealSlateCallback>();
  Callback->Library = RustealCurrentLibrary();
  Callback->CallbackId = CallbackId;
  GSlateCallbacks.Add(&Callback.Get());
  return Callback;
}

static FText PropText(const RustealSlateProps *Props) {
  return FText::FromString(RustealUtf8ToFString(Props->text, Props->text_len));
}

static TAttribute<FText>
TextAttribute(const RustealSlateProps *Props,
              const TSharedPtr<FRustealSlateCallback> &Callback) {
  if (!Callback) {
    return PropText(Props);
  }

  return TAttribute<FText>::CreateLambda([Callback]() {
    FString Out;
    Callback->Invoke(reinterpret_cast<uint8 *>(&Out));
    return FText::FromString(Out);
  });
}

static TAttribute<bool>
BoolAttribute(bool Default, const TSharedPtr<FRustealSlateCallback> &Callback) {
  if (!Callback) {
    return Default;
  }

  return TAttribute<bool>::CreateLambda([Callback]() {
    bool Out = false;
    Callback->Invoke(reinterpret_cast<uint8 *>(&Out));
    return Out;
  });
}

static void InvokeWithText(const TSharedPtr<FRustealSlateCallback> &Callback,
                           const FText &Text) {
  if (!Callback) {
    return;
  }

  FTCHARToUTF8 Utf8(*Text.ToString());
  FRustealSlateTextArg Arg{reinterpret_cast<const uint8 *>(Utf8.Get()),
                           static_cast<uint32>(Utf8.Length())};
  Callback->Invoke(reinterpret_cast<uint8 *>(&Arg));
}

static TSharedRef<SWidget> MakeWidget(ERustealSlateKind Kind,
                                      const RustealSlateProps *Props) {
  const TSharedPtr<FRustealSlateCallback> TextCallback =
      MakeCallback(Props->text_callback);
  const TSharedPtr<FRustealSlateCallback> EnabledCallback =
      MakeCallback(Props->enabled_callback);
  const TSharedPtr<FRustealSlateCallback> Event = MakeCallback(Props->on_event);
  const TSharedPtr<FRustealSlateCallback> Commit =
      MakeCallback(Props->on_commit);
  const TAttribute<bool> Enabled = BoolAttribute(true, EnabledCallback);

  switch (Kind) {
  case ERustealSlateKind::TextBlock:
    return SNew(STextBlock)
        .Text(TextAttribute(Props, TextCallback))
        .IsEnabled(Enabled);

  case ERustealSlateKind::Button: {
    TSharedRef<SButton> Button =
        SNew(SButton).IsEnabled(Enabled).OnClicked_Lambda([Event]() {
          if (Event) {
            Event->Invoke(nullptr);
          }

          return FReply::Handled();
        });

    if (Props->text_len > 0 || TextCallback) {
      Button->SetContent(
          SNew(STextBlock).Text(TextAttribute(Props, TextCallback)));
    }

    return Button;
  }

  case ERustealSlateKind::CheckBox:
    return SNew(SCheckBox)
        .IsEnabled(Enabled)
        .IsChecked(Props->checked ? ECheckBoxState::Checked
                                  : ECheckBoxState::Unchecked)
        .OnCheckStateChanged_Lambda([Event](ECheckBoxState State) {
          if (Event) {
            bool bChecked = State == ECheckBoxState::Checked;
            Event->Invoke(reinterpret_cast<uint8 *>(&bChecked));
          }
        });

  case ERustealSlateKind::EditableTextBox:
    return SNew(SEditableTextBox)
        .IsEnabled(Enabled)
        .Text(PropText(Props))
        .HintText(FText::FromString(
            RustealUtf8ToFString(Props->hint, Props->hint_len)))
        .OnTextChanged_Lambda(
            [Event](const FText &Text) { InvokeWithText(Event, Text); })
        .OnTextCommitted_Lambda([Commit](const FText &Text, ETextCommit::Type) {
          InvokeWithText(Commit, Text);
        });

  case ERustealSlateKind::SpinBox:
    return SNew(SSpinBox<float>)
        .IsEnabled(Enabled)
        .MinValue(Props->min)
        .MaxValue(Props->max)
        .Value(Props->value)
        .OnValueChanged_Lambda([Event](float Value) {
          if (Event) {
            Event->Invoke(reinterpret_cast<uint8 *>(&Value));
          }
        });

  case ERustealSlateKind::VerticalBox:
    return SNew(SVerticalBox).IsEnabled(Enabled);

  case ERustealSlateKind::HorizontalBox:
    return SNew(SHorizontalBox).IsEnabled(Enabled);

  case ERustealSlateKind::ScrollBox:
    return SNew(SScrollBox).IsEnabled(Enabled);

  case ERustealSlateKind::Border:
    return SNew(SBorder).IsEnabled(Enabled).Padding(
        FMargin(Props->padding[0], Props->padding[1], Props->padding[2],
                Props->padding[3]));

  case ERustealSlateKind::Spacer:
    return SNew(SSpacer).Size(FVector2D(Props->min, Props->max));

  case ERustealSlateKind::Adopted:
    break;
  }

  return SNullWidget::NullWidget;
}

static uint64 SlateNewImpl(uint32 Kind, const RustealSlateProps *Props) {
  if (!Props || Kind < 1 || Kind > 10 || !FSlateApplication::IsInitialized()) {
    RustealFail(ERustealErrorCode::InvalidOperation,
                TEXT("slate: no such widget, or Slate is not running"));
    return 0;
  }

  const ERustealSlateKind WidgetKind = static_cast<ERustealSlateKind>(Kind);
  const uint64 Id = GNextWidgetId++;

  GSlateWidgets.Add(Id,
                    FRustealSlateWidget{MakeWidget(WidgetKind, Props),
                                        WidgetKind, RustealCurrentLibrary()});
  return Id;
}

static EHorizontalAlignment HAlign(uint8 Value) {
  return Value <= HAlign_Right ? static_cast<EHorizontalAlignment>(Value)
                               : HAlign_Fill;
}

static EVerticalAlignment VAlign(uint8 Value) {
  return Value <= VAlign_Bottom ? static_cast<EVerticalAlignment>(Value)
                                : VAlign_Fill;
}

static ERustealErrorCode SlateAddChildImpl(uint64 Parent, uint64 Child,
                                           const RustealSlateSlot *Slot) {
  FRustealSlateWidget *ParentEntry = GSlateWidgets.Find(Parent);
  FRustealSlateWidget *ChildEntry = GSlateWidgets.Find(Child);

  if (!ParentEntry || !ChildEntry || !Slot) {
    return RustealFail(ERustealErrorCode::ObjectDestroyed,
                       TEXT("slate: a widget was released"));
  }

  const TSharedRef<SWidget> Widget = ChildEntry->Widget;
  const FMargin Padding(Slot->padding[0], Slot->padding[1], Slot->padding[2],
                        Slot->padding[3]);

  switch (ParentEntry->Kind) {
  case ERustealSlateKind::VerticalBox: {
    SVerticalBox::FScopedWidgetSlotArguments Added =
        StaticCastSharedRef<SVerticalBox>(ParentEntry->Widget)->AddSlot();
    Added.Padding(Padding)
        .HAlign(HAlign(Slot->h_align))
        .VAlign(VAlign(Slot->v_align));

    if (Slot->fill > 0.0f) {
      Added.FillHeight(Slot->fill);
    } else {
      Added.AutoHeight();
    }

    Added[Widget];
    break;
  }

  case ERustealSlateKind::HorizontalBox: {
    SHorizontalBox::FScopedWidgetSlotArguments Added =
        StaticCastSharedRef<SHorizontalBox>(ParentEntry->Widget)->AddSlot();
    Added.Padding(Padding)
        .HAlign(HAlign(Slot->h_align))
        .VAlign(VAlign(Slot->v_align));

    if (Slot->fill > 0.0f) {
      Added.FillWidth(Slot->fill);
    } else {
      Added.AutoWidth();
    }

    Added[Widget];
    break;
  }

  case ERustealSlateKind::ScrollBox:
    StaticCastSharedRef<SScrollBox>(ParentEntry->Widget)
        ->AddSlot()
        .Padding(Padding)
        .HAlign(HAlign(Slot->h_align))
        .VAlign(VAlign(Slot->v_align))[Widget];
    break;

  case ERustealSlateKind::Border:
    StaticCastSharedRef<SBorder>(ParentEntry->Widget)->SetContent(Widget);
    break;

  case ERustealSlateKind::Button:
    StaticCastSharedRef<SButton>(ParentEntry->Widget)->SetContent(Widget);
    break;

  case ERustealSlateKind::CheckBox:
    StaticCastSharedRef<SCheckBox>(ParentEntry->Widget)->SetContent(Widget);
    break;

  default:
    return RustealFail(ERustealErrorCode::InvalidOperation,
                       TEXT("slate: this widget has no slot for a child"));
  }

  return ERustealErrorCode::Ok;
}

static void SlateReleaseImpl(uint64 Id) { GSlateWidgets.Remove(Id); }

static void SlateWriteTextImpl(uint8 *Out, const uint8 *Utf8, uint32 Len) {
  if (Out) {
    *reinterpret_cast<FString *>(Out) = RustealUtf8ToFString(Utf8, Len);
  }
}

static bool SlateSimulateClickImpl(uint64 Id) {
  FRustealSlateWidget *Entry = GSlateWidgets.Find(Id);

  if (!Entry || Entry->Kind != ERustealSlateKind::Button) {
    return false;
  }

  StaticCastSharedRef<SButton>(Entry->Widget)->SimulateClick();
  return true;
}

static ERustealErrorCode SlateGetTextImpl(uint64 Id, uint8 *Buffer,
                                          uint32 Capacity, uint32 *OutLen) {
  FRustealSlateWidget *Entry = GSlateWidgets.Find(Id);

  if (!Entry) {
    return RustealFail(ERustealErrorCode::ObjectDestroyed);
  }

  FText Text;

  switch (Entry->Kind) {
  case ERustealSlateKind::TextBlock:
    Text = StaticCastSharedRef<STextBlock>(Entry->Widget)->GetText();
    break;
  case ERustealSlateKind::EditableTextBox:
    Text = StaticCastSharedRef<SEditableTextBox>(Entry->Widget)->GetText();
    break;
  default:
    return RustealFail(ERustealErrorCode::InvalidOperation,
                       TEXT("slate: this widget has no text"));
  }

  FTCHARToUTF8 Utf8(*Text.ToString());
  const uint32 Len = static_cast<uint32>(Utf8.Length());

  if (OutLen) {
    *OutLen = Len;
  }

  if (Buffer && Capacity >= Len) {
    FMemory::Memcpy(Buffer, Utf8.Get(), Len);
  }

  return ERustealErrorCode::Ok;
}

static ERustealErrorCode SlateRegisterTabImpl(const uint8 *Name, uint32 NameLen,
                                              const uint8 *Label,
                                              uint32 LabelLen,
                                              uint64 CallbackId) {
  if (!FSlateApplication::IsInitialized()) {
    return RustealFail(ERustealErrorCode::InvalidOperation,
                       TEXT("slate: Slate is not running"));
  }

  const FName TabName(RustealUtf8ToFString(Name, NameLen));
  const TSharedPtr<FRustealSlateCallback> Builder = MakeCallback(CallbackId);

  if (!Builder) {
    return RustealFail(ERustealErrorCode::InvalidOperation);
  }

  if (FGlobalTabmanager::Get()->HasTabSpawner(TabName)) {
    FGlobalTabmanager::Get()->UnregisterNomadTabSpawner(TabName);
  }

  FGlobalTabmanager::Get()
      ->RegisterNomadTabSpawner(
          TabName, FOnSpawnTab::CreateLambda([Builder](const FSpawnTabArgs &) {
            uint64 WidgetId = 0;
            Builder->Invoke(reinterpret_cast<uint8 *>(&WidgetId));

            TSharedRef<SWidget> Content = SNullWidget::NullWidget;

            if (const FRustealSlateWidget *Built =
                    GSlateWidgets.Find(WidgetId)) {
              Content = Built->Widget;
              GSlateWidgets.Remove(WidgetId);
            }

            return SNew(SDockTab).TabRole(ETabRole::NomadTab)[Content];
          }))
      .SetDisplayName(FText::FromString(RustealUtf8ToFString(Label, LabelLen)));

  GSlateTabs.Add(TabName, Builder.ToSharedRef());
  return ERustealErrorCode::Ok;
}

static bool SlateInvokeTabImpl(const uint8 *Name, uint32 NameLen) {
  if (!FSlateApplication::IsInitialized()) {
    return false;
  }

  const FName TabName(RustealUtf8ToFString(Name, NameLen));
  return FGlobalTabmanager::Get()->TryInvokeTab(FTabId(TabName)).IsValid();
}

static void SlateUnregisterTabImpl(const uint8 *Name, uint32 NameLen) {
  const FName TabName(RustealUtf8ToFString(Name, NameLen));

  if (FSlateApplication::IsInitialized()) {
    if (TSharedPtr<SDockTab> Live =
            FGlobalTabmanager::Get()->FindExistingLiveTab(FTabId(TabName))) {
      Live->RequestCloseTab();
    }

    FGlobalTabmanager::Get()->UnregisterNomadTabSpawner(TabName);
  }

  GSlateTabs.Remove(TabName);
}

static uint32 SlateLiveWidgetsImpl() {
  return static_cast<uint32>(GSlateWidgets.Num());
}

static uint32 SlateLiveCallbacksImpl() {
  return static_cast<uint32>(GSlateCallbacks.Num());
}

void RustealSlateForgetLibrary(FRustealLibrary *Library) {
  for (FRustealSlateCallback *Callback : GSlateCallbacks) {
    if (Callback->Library == Library) {
      Callback->Library = nullptr;
    }
  }

  TArray<FName> Tabs;

  for (const TPair<FName, TSharedRef<FRustealSlateCallback>> &Tab :
       GSlateTabs) {
    if (!Tab.Value->Library) {
      Tabs.Add(Tab.Key);
    }
  }

  for (const FName &Tab : Tabs) {
    if (FSlateApplication::IsInitialized()) {
      if (TSharedPtr<SDockTab> Live =
              FGlobalTabmanager::Get()->FindExistingLiveTab(FTabId(Tab))) {
        Live->RequestCloseTab();
      }

      FGlobalTabmanager::Get()->UnregisterNomadTabSpawner(Tab);
    }

    GSlateTabs.Remove(Tab);
  }

  TArray<uint64> Widgets;

  for (const TPair<uint64, FRustealSlateWidget> &Entry : GSlateWidgets) {
    if (Entry.Value.Library == Library) {
      Widgets.Add(Entry.Key);
    }
  }

  for (uint64 Id : Widgets) {
    GSlateWidgets.Remove(Id);
  }
}

uint64 RustealSlate_New(uint32 Kind, const RustealSlateProps *Props) {
  return SlateNewImpl(Kind, Props);
}

ERustealErrorCode RustealSlate_AddChild(uint64 Parent, uint64 Child,
                                        const RustealSlateSlot *Slot) {
  return SlateAddChildImpl(Parent, Child, Slot);
}

void RustealSlate_Release(uint64 Id) { SlateReleaseImpl(Id); }

void RustealSlate_WriteText(uint8 *Out, const uint8 *Utf8, uint32 Len) {
  SlateWriteTextImpl(Out, Utf8, Len);
}

bool RustealSlate_SimulateClick(uint64 Id) {
  return SlateSimulateClickImpl(Id);
}

ERustealErrorCode RustealSlate_GetText(uint64 Id, uint8 *Buffer,
                                       uint32 Capacity, uint32 *OutLen) {
  return SlateGetTextImpl(Id, Buffer, Capacity, OutLen);
}

ERustealErrorCode RustealSlate_RegisterTab(const uint8 *Name, uint32 NameLen,
                                           const uint8 *Label, uint32 LabelLen,
                                           uint64 CallbackId) {
  return SlateRegisterTabImpl(Name, NameLen, Label, LabelLen, CallbackId);
}

bool RustealSlate_InvokeTab(const uint8 *Name, uint32 NameLen) {
  return SlateInvokeTabImpl(Name, NameLen);
}

void RustealSlate_UnregisterTab(const uint8 *Name, uint32 NameLen) {
  SlateUnregisterTabImpl(Name, NameLen);
}

uint32 RustealSlate_LiveWidgets() { return SlateLiveWidgetsImpl(); }

uint32 RustealSlate_LiveCallbacks() { return SlateLiveCallbacksImpl(); }

uint64 RustealSlateAdopt(const TSharedRef<SWidget> &Widget) {
  const uint64 Id = GNextWidgetId++;
  GSlateWidgets.Add(Id, FRustealSlateWidget{Widget, ERustealSlateKind::Adopted,
                                            RustealCurrentLibrary()});
  return Id;
}

TSharedPtr<SWidget> RustealSlateTake(uint64 Id) {
  if (const FRustealSlateWidget *Entry = GSlateWidgets.Find(Id)) {
    TSharedRef<SWidget> Widget = Entry->Widget;
    GSlateWidgets.Remove(Id);
    return Widget;
  }

  return nullptr;
}
