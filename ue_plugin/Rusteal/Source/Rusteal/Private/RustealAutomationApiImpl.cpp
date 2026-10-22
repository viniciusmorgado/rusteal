#include "Misc/AutomationTest.h"
#include "RustealApiTable.h"
#include "RustealErrors.h"
#include "RustealLibrary.h"
#include "RustealModule.h"
#include "RustealStrings.h"

#if WITH_AUTOMATION_TESTS

class FRustealAutomationTest : public FAutomationTestBase {
public:
  FRustealAutomationTest(const FString &InName, EAutomationTestFlags InFlags,
                         FRustealLibrary *InLibrary, uint64 InCallbackId)
      : FAutomationTestBase(InName, false), Flags(InFlags), Library(InLibrary),
        CallbackId(InCallbackId) {}

  virtual EAutomationTestFlags GetTestFlags() const override { return Flags; }

  virtual uint32 GetRequiredDeviceNum() const override { return 1; }

  virtual bool IsStressTest() const { return false; }

  virtual FString GetBeautifiedTestName() const override { return TestName; }

protected:
  virtual void GetTests(TArray<FString> &OutBeautifiedNames,
                        TArray<FString> &OutTestCommands) const override {
    OutBeautifiedNames.Add(GetBeautifiedTestName());
    OutTestCommands.Add(FString());
  }

  virtual bool RunTest(const FString &Parameters) override {
    if (!Library || !Library->IsLoaded()) {
      AddError(TEXT("the Rust library of this test is no longer loaded"));
      return false;
    }

    RustealCallLibrary(Library, [this](const FRustealRustCallbacks &Cb) {
      Cb.invoke_delegate_callback(CallbackId, nullptr);
    });

    return !HasAnyErrors();
  }

private:
  EAutomationTestFlags Flags;
  FRustealLibrary *Library;
  uint64 CallbackId;
};

static TMap<FRustealLibrary *, TArray<TUniquePtr<FRustealAutomationTest>>>
    GAutomationTests;

#endif

ERustealErrorCode RustealAutomation_RegisterTest(const uint8 *Name,
                                                 uint32 NameLen,
                                                 uint32 Contexts,
                                                 uint64 CallbackId) {
#if WITH_AUTOMATION_TESTS
  const FString TestName = RustealUtf8ToFString(Name, NameLen);

  if (TestName.IsEmpty()) {
    return RustealFail(ERustealErrorCode::NullArgument,
                       TEXT("an automation test has a name"));
  }

  EAutomationTestFlags Flags = static_cast<EAutomationTestFlags>(Contexts) &
                               EAutomationTestFlags_ApplicationContextMask;

  if (Flags == EAutomationTestFlags::None) {
    Flags = EAutomationTestFlags_ApplicationContextMask;
  }

  FRustealLibrary *Library = RustealCurrentLibrary();
  TArray<TUniquePtr<FRustealAutomationTest>> &Tests =
      GAutomationTests.FindOrAdd(Library);

  Tests.RemoveAll([&TestName](const TUniquePtr<FRustealAutomationTest> &Test) {
    return Test->GetBeautifiedTestName() == TestName;
  });

  Tests.Add(MakeUnique<FRustealAutomationTest>(
      TestName, Flags | EAutomationTestFlags::ProductFilter, Library,
      CallbackId));

  return ERustealErrorCode::Ok;
#else
  return RustealFail(ERustealErrorCode::InvalidOperation,
                     TEXT("automation tests are not in this build"));
#endif
}

void RustealAutomation_AddMessage(uint32 Kind, const uint8 *Text,
                                  uint32 TextLen) {
#if WITH_AUTOMATION_TESTS
  FAutomationTestBase *Test = FAutomationTestFramework::Get().GetCurrentTest();

  if (!Test) {
    UE_LOG(LogRusteal, Warning,
           TEXT("[Rusteal] automation: no test is running for: %s"),
           *RustealUtf8ToFString(Text, TextLen));
    return;
  }

  const FString Message = RustealUtf8ToFString(Text, TextLen);

  switch (Kind) {
  case 0:
    Test->AddError(Message);
    break;
  case 1:
    Test->AddWarning(Message);
    break;
  default:
    Test->AddInfo(Message);
    break;
  }
#endif
}

void RustealAutomationForgetLibrary(FRustealLibrary *Library) {
#if WITH_AUTOMATION_TESTS
  GAutomationTests.Remove(Library);
#endif
}
