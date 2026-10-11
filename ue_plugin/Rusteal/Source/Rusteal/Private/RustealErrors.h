#pragma once

#include "CoreMinimal.h"
#include "RustealApiTable.h"

class FProperty;
class UObject;

ERustealErrorCode RustealFail(ERustealErrorCode Code,
                              const FString &Message = FString());

ERustealErrorCode RustealFailType(const void *Property, const TCHAR *Expected);

FString RustealDescribe(const UObject *Object);

FString RustealDescribe(const FProperty *Property);

bool RustealTakeErrorMessage(ERustealErrorCode Code, uint8 *Buf, uint32 BufLen,
                             uint32 *OutLen);

void RustealReport(const FRustealErrorReport *Report);

FString RustealTakePendingScriptError();
