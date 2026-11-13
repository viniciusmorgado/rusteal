#include "HttpModule.h"
#include "Interfaces/IHttpRequest.h"
#include "Interfaces/IHttpResponse.h"
#include "RustealApiTable.h"
#include "RustealErrors.h"
#include "RustealLibrary.h"
#include "RustealStrings.h"

namespace {
struct FRustealHttpCall {
  FRustealLibrary *Library = nullptr;
  FHttpRequestPtr Request;
  FHttpResponsePtr Response;
  bool bSucceeded = false;
  bool bDone = false;
};

FCriticalSection GHttpLock;
TMap<uint64, FRustealHttpCall> GHttpCalls;
} // namespace

ERustealErrorCode RustealHttp_Send(const uint8 *Verb, uint32 VerbLen,
                                   const uint8 *Url, uint32 UrlLen,
                                   const uint8 *Headers, uint32 HeadersLen,
                                   const uint8 *Body, uint32 BodyLen,
                                   float TimeoutSeconds, uint64 CallbackId) {
  if (!FModuleManager::Get().IsModuleLoaded("HTTP") &&
      !FModuleManager::Get().LoadModule("HTTP")) {
    return RustealFail(ERustealErrorCode::InvalidOperation,
                       TEXT("http: the HTTP module is not available"));
  }

  const TSharedRef<IHttpRequest, ESPMode::ThreadSafe> Request =
      FHttpModule::Get().CreateRequest();

  Request->SetVerb(RustealUtf8ToFString(Verb, VerbLen));
  Request->SetURL(RustealUtf8ToFString(Url, UrlLen));

  TArray<FString> Lines;
  RustealUtf8ToFString(Headers, HeadersLen).ParseIntoArray(Lines, TEXT("\n"), true);

  for (const FString &Line : Lines) {
    FString Name;
    FString Value;

    if (Line.Split(TEXT(":"), &Name, &Value)) {
      Request->SetHeader(Name.TrimStartAndEnd(), Value.TrimStartAndEnd());
    }
  }

  if (BodyLen > 0) {
    Request->SetContent(TArray<uint8>(Body, BodyLen));
  }

  if (TimeoutSeconds > 0.0f) {
    Request->SetTimeout(TimeoutSeconds);
  }

  FRustealLibrary *Library = RustealCurrentLibrary();

  Request->OnProcessRequestComplete().BindLambda(
      [Library, CallbackId](FHttpRequestPtr, FHttpResponsePtr Response,
                            bool bSucceeded) {
        {
          FScopeLock Lock(&GHttpLock);
          FRustealHttpCall *Call = GHttpCalls.Find(CallbackId);

          if (!Call) {
            return;
          }

          Call->Response = Response;
          Call->bSucceeded = bSucceeded && Response.IsValid();
          Call->bDone = true;
        }

        RustealCallLibrary(Library, [CallbackId](const FRustealRustCallbacks &Cb) {
          Cb.invoke_delegate_callback(CallbackId, nullptr);
        });
      });

  {
    FScopeLock Lock(&GHttpLock);
    FRustealHttpCall &Call = GHttpCalls.Add(CallbackId);
    Call.Library = Library;
    Call.Request = Request;
  }

  if (!Request->ProcessRequest()) {
    FScopeLock Lock(&GHttpLock);
    GHttpCalls.Remove(CallbackId);

    return RustealFail(ERustealErrorCode::InvalidOperation,
                       TEXT("http: the request could not start"));
  }

  return ERustealErrorCode::Ok;
}

ERustealErrorCode RustealHttp_Finish(uint64 CallbackId, int32 *OutStatus,
                                     uint8 *Headers, uint32 HeadersCapacity,
                                     uint32 *OutHeadersLen, uint8 *Body,
                                     uint32 BodyCapacity, uint32 *OutBodyLen,
                                     bool *OutSucceeded) {
  FScopeLock Lock(&GHttpLock);
  FRustealHttpCall *Call = GHttpCalls.Find(CallbackId);

  if (!Call || !Call->bDone) {
    return RustealFail(ERustealErrorCode::InvalidOperation,
                       TEXT("http: no finished request"));
  }

  const FString HeaderText =
      Call->Response.IsValid()
          ? FString::Join(Call->Response->GetAllHeaders(), TEXT("\n"))
          : FString();
  const FTCHARToUTF8 HeaderUtf8(*HeaderText);

  const TArray<uint8> Empty;
  const TArray<uint8> &Content =
      Call->Response.IsValid() ? Call->Response->GetContent() : Empty;

  *OutStatus = Call->Response.IsValid() ? Call->Response->GetResponseCode() : 0;
  *OutHeadersLen = static_cast<uint32>(HeaderUtf8.Length());
  *OutBodyLen = static_cast<uint32>(Content.Num());
  *OutSucceeded = Call->bSucceeded;

  if (*OutHeadersLen > HeadersCapacity || *OutBodyLen > BodyCapacity) {
    return RustealFail(ERustealErrorCode::BufferTooSmall);
  }

  FMemory::Memcpy(Headers, HeaderUtf8.Get(), *OutHeadersLen);
  FMemory::Memcpy(Body, Content.GetData(), *OutBodyLen);
  GHttpCalls.Remove(CallbackId);
  return ERustealErrorCode::Ok;
}

ERustealErrorCode RustealHttp_Cancel(uint64 CallbackId) {
  FHttpRequestPtr Request;

  {
    FScopeLock Lock(&GHttpLock);
    FRustealHttpCall Call;

    if (GHttpCalls.RemoveAndCopyValue(CallbackId, Call)) {
      Request = Call.Request;
    }
  }

  if (Request.IsValid()) {
    Request->OnProcessRequestComplete().Unbind();
    Request->CancelRequest();
  }

  return ERustealErrorCode::Ok;
}

uint32 RustealHttp_InProgress() {
  FScopeLock Lock(&GHttpLock);
  return static_cast<uint32>(GHttpCalls.Num());
}

void RustealHttpForgetLibrary(FRustealLibrary *Library) {
  TArray<uint64> Owned;

  {
    FScopeLock Lock(&GHttpLock);

    for (const TPair<uint64, FRustealHttpCall> &Entry : GHttpCalls) {
      if (Entry.Value.Library == Library) {
        Owned.Add(Entry.Key);
      }
    }
  }

  for (uint64 Id : Owned) {
    RustealHttp_Cancel(Id);
  }
}
