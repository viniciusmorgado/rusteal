#include "RustealApiTable.h"
#include "RustealErrors.h"
#include "RustealLibrary.h"
#include "RustealStrings.h"

uint64 RustealSlateAdopt(const TSharedRef<SWidget> &Widget);
TSharedPtr<SWidget> RustealSlateTake(uint64 Id);

#if WITH_EDITOR
#include "DetailCategoryBuilder.h"
#include "DetailLayoutBuilder.h"
#include "DetailWidgetRow.h"
#include "IDetailCustomization.h"
#include "IDetailsView.h"
#include "Modules/ModuleManager.h"
#include "PropertyEditorModule.h"

struct FRustealDetailsCustomizer {
  FRustealLibrary *Library = nullptr;
  uint64 CallbackId = 0;
  FName ClassName;

  ~FRustealDetailsCustomizer() {
    RustealCallLibrary(Library, [this](const FRustealRustCallbacks &Cb) {
      Cb.release_delegate_callback(CallbackId);
    });
  }
};

static TMap<FName, TSharedRef<FRustealDetailsCustomizer>> GCustomizers;
static TMap<uint64, IDetailLayoutBuilder *> GLayouts;
static uint64 GNextLayout = 1;

class FRustealDetailCustomization : public IDetailCustomization {
public:
  explicit FRustealDetailCustomization(
      TSharedRef<FRustealDetailsCustomizer> InCustomizer)
      : Customizer(InCustomizer) {}

  virtual void CustomizeDetails(IDetailLayoutBuilder &Builder) override {
    uint64 Layout = GNextLayout++;
    GLayouts.Add(Layout, &Builder);

    RustealCallLibrary(
        Customizer->Library, [this, &Layout](const FRustealRustCallbacks &Cb) {
          Cb.invoke_delegate_callback(Customizer->CallbackId,
                                      reinterpret_cast<uint8 *>(&Layout));
        });

    GLayouts.Remove(Layout);
  }

private:
  TSharedRef<FRustealDetailsCustomizer> Customizer;
};

static FPropertyEditorModule &PropertyEditor() {
  return FModuleManager::LoadModuleChecked<FPropertyEditorModule>(
      TEXT("PropertyEditor"));
}

static ERustealErrorCode DetailsCustomizeClassImpl(RustealUClassHandle Class,
                                                   uint64 CallbackId) {
  UClass *Target = static_cast<UClass *>(Class.ptr);

  if (!Target || CallbackId == 0) {
    return RustealFail(ERustealErrorCode::InvalidOperation);
  }

  TSharedRef<FRustealDetailsCustomizer> Customizer =
      MakeShared<FRustealDetailsCustomizer>();
  Customizer->Library = RustealCurrentLibrary();
  Customizer->CallbackId = CallbackId;
  Customizer->ClassName = Target->GetFName();

  PropertyEditor().RegisterCustomClassLayout(
      Customizer->ClassName,
      FOnGetDetailCustomizationInstance::CreateLambda([Customizer]() {
        return MakeShared<FRustealDetailCustomization>(Customizer);
      }));
  PropertyEditor().NotifyCustomizationModuleChanged();

  GCustomizers.Add(Customizer->ClassName, Customizer);
  return ERustealErrorCode::Ok;
}

static void DetailsUnregisterClassImpl(RustealUClassHandle Class) {
  UClass *Target = static_cast<UClass *>(Class.ptr);

  if (!Target || !GCustomizers.Contains(Target->GetFName())) {
    return;
  }

  if (FModuleManager::Get().IsModuleLoaded(TEXT("PropertyEditor"))) {
    PropertyEditor().UnregisterCustomClassLayout(Target->GetFName());
    PropertyEditor().NotifyCustomizationModuleChanged();
  }

  GCustomizers.Remove(Target->GetFName());
}

static IDetailLayoutBuilder *FindLayout(uint64 Layout) {
  IDetailLayoutBuilder **Found = GLayouts.Find(Layout);
  return Found ? *Found : nullptr;
}

static ERustealErrorCode DetailsAddRowImpl(uint64 Layout, const uint8 *Category,
                                           uint32 CategoryLen,
                                           uint64 NameWidget,
                                           uint64 ValueWidget) {
  IDetailLayoutBuilder *Builder = FindLayout(Layout);

  if (!Builder) {
    return RustealFail(ERustealErrorCode::InvalidOperation,
                       TEXT("details: the layout is only there while the "
                            "customization runs"));
  }

  const FString CategoryName = RustealUtf8ToFString(Category, CategoryLen);
  const TSharedPtr<SWidget> Name = RustealSlateTake(NameWidget);
  const TSharedPtr<SWidget> Value = RustealSlateTake(ValueWidget);

  IDetailCategoryBuilder &CategoryBuilder =
      Builder->EditCategory(FName(*CategoryName));

  CategoryBuilder.AddCustomRow(FText::FromString(CategoryName))
      .NameContent()[Name ? Name.ToSharedRef() : SNullWidget::NullWidget]
      .ValueContent()[Value ? Value.ToSharedRef() : SNullWidget::NullWidget];

  return ERustealErrorCode::Ok;
}

static ERustealErrorCode DetailsHidePropertyImpl(uint64 Layout,
                                                 const uint8 *Property,
                                                 uint32 PropertyLen) {
  IDetailLayoutBuilder *Builder = FindLayout(Layout);

  if (!Builder) {
    return RustealFail(ERustealErrorCode::InvalidOperation);
  }

  const FName Name(RustealUtf8ToFString(Property, PropertyLen));
  TSharedRef<IPropertyHandle> Handle = Builder->GetProperty(Name);

  if (!Handle->IsValidHandle()) {
    return RustealFail(ERustealErrorCode::PropertyNotFound);
  }

  Builder->HideProperty(Handle);
  return ERustealErrorCode::Ok;
}

static uint32 DetailsObjectsImpl(uint64 Layout, RustealUObjectHandle *Out,
                                 uint32 Capacity) {
  IDetailLayoutBuilder *Builder = FindLayout(Layout);

  if (!Builder) {
    return 0;
  }

  TArray<TWeakObjectPtr<UObject>> Objects;
  Builder->GetObjectsBeingCustomized(Objects);

  for (int32 Index = 0;
       Out && Index < Objects.Num() && Index < static_cast<int32>(Capacity);
       ++Index) {
    Out[Index] = RustealUObjectHandle{Objects[Index].Get()};
  }

  return static_cast<uint32>(Objects.Num());
}

static uint64 DetailsViewImpl(RustealUObjectHandle Object) {
  UObject *Target = static_cast<UObject *>(Object.ptr);

  if (!IsValid(Target)) {
    RustealFail(ERustealErrorCode::ObjectDestroyed);
    return 0;
  }

  FDetailsViewArgs Args;
  Args.bAllowSearch = true;
  Args.bHideSelectionTip = true;

  TSharedRef<IDetailsView> View = PropertyEditor().CreateDetailView(Args);
  View->SetObject(Target);
  return RustealSlateAdopt(View);
}

void RustealDetailsForgetLibrary(FRustealLibrary *Library) {
  TArray<FName> Owned;

  for (const TPair<FName, TSharedRef<FRustealDetailsCustomizer>> &Entry :
       GCustomizers) {
    if (Entry.Value->Library == Library) {
      Entry.Value->Library = nullptr;
      Owned.Add(Entry.Key);
    }
  }

  for (const FName &Name : Owned) {
    if (FModuleManager::Get().IsModuleLoaded(TEXT("PropertyEditor"))) {
      PropertyEditor().UnregisterCustomClassLayout(Name);
    }

    GCustomizers.Remove(Name);
  }

  if (!Owned.IsEmpty() &&
      FModuleManager::Get().IsModuleLoaded(TEXT("PropertyEditor"))) {
    PropertyEditor().NotifyCustomizationModuleChanged();
  }
}
#else
static ERustealErrorCode EditorOnly() {
  return RustealFail(ERustealErrorCode::InvalidOperation,
                     TEXT("details: the details panel is in the editor only"));
}

static ERustealErrorCode DetailsCustomizeClassImpl(RustealUClassHandle,
                                                   uint64) {
  return EditorOnly();
}

static void DetailsUnregisterClassImpl(RustealUClassHandle) {}

static ERustealErrorCode DetailsAddRowImpl(uint64, const uint8 *, uint32,
                                           uint64, uint64) {
  return EditorOnly();
}

static ERustealErrorCode DetailsHidePropertyImpl(uint64, const uint8 *,
                                                 uint32) {
  return EditorOnly();
}

static uint32 DetailsObjectsImpl(uint64, RustealUObjectHandle *, uint32) {
  return 0;
}

static uint64 DetailsViewImpl(RustealUObjectHandle) {
  EditorOnly();
  return 0;
}

void RustealDetailsForgetLibrary(FRustealLibrary *) {}
#endif

ERustealErrorCode RustealDetails_CustomizeClass(RustealUClassHandle Class,
                                                uint64 CallbackId) {
  return DetailsCustomizeClassImpl(Class, CallbackId);
}

void RustealDetails_UnregisterClass(RustealUClassHandle Class) {
  DetailsUnregisterClassImpl(Class);
}

ERustealErrorCode RustealDetails_AddRow(uint64 Layout, const uint8 *Category,
                                        uint32 CategoryLen, uint64 NameWidget,
                                        uint64 ValueWidget) {
  return DetailsAddRowImpl(Layout, Category, CategoryLen, NameWidget,
                           ValueWidget);
}

ERustealErrorCode RustealDetails_HideProperty(uint64 Layout,
                                              const uint8 *Property,
                                              uint32 PropertyLen) {
  return DetailsHidePropertyImpl(Layout, Property, PropertyLen);
}

uint32 RustealDetails_Objects(uint64 Layout, RustealUObjectHandle *Out,
                              uint32 Capacity) {
  return DetailsObjectsImpl(Layout, Out, Capacity);
}

uint64 RustealDetails_View(RustealUObjectHandle Object) {
  return DetailsViewImpl(Object);
}
