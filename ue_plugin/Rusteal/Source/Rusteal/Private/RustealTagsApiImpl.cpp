#include "GameplayTagContainer.h"
#include "GameplayTagsManager.h"
#include "RustealApiTable.h"
#include "RustealErrors.h"
#include "RustealStrings.h"

ERustealErrorCode RustealTags_AddNative(const uint8 *Name, uint32 NameLen,
                                        const uint8 *Comment,
                                        uint32 CommentLen) {
  const FString TagName = RustealUtf8ToFString(Name, NameLen);

  if (TagName.IsEmpty()) {
    return RustealFail(ERustealErrorCode::InvalidOperation,
                       TEXT("gameplay tags: a tag has a name"));
  }

  if (TagName.StartsWith(TEXT(".")) || TagName.EndsWith(TEXT(".")) ||
      TagName.Contains(TEXT(".."))) {
    return RustealFail(
        ERustealErrorCode::InvalidOperation,
        FString::Printf(TEXT("gameplay tags: %s has an empty part"), *TagName));
  }

  FText Error;

  if (!UGameplayTagsManager::Get().IsValidGameplayTagString(TagName, &Error)) {
    return RustealFail(ERustealErrorCode::InvalidOperation,
                       FString::Printf(TEXT("gameplay tags: %s: %s"), *TagName,
                                       *Error.ToString()));
  }

  const FGameplayTag Tag = UGameplayTagsManager::Get().AddNativeGameplayTag(
      FName(*TagName), RustealUtf8ToFString(Comment, CommentLen));

  return Tag.IsValid() ? ERustealErrorCode::Ok
                       : RustealFail(ERustealErrorCode::InvalidOperation);
}

bool RustealTags_IsRegistered(const uint8 *Name, uint32 NameLen) {
  return UGameplayTagsManager::Get()
      .RequestGameplayTag(FName(*RustealUtf8ToFString(Name, NameLen)), false)
      .IsValid();
}
