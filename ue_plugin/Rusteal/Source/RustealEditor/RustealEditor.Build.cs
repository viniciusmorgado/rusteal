namespace UnrealBuildTool.Rules;

public class RustealEditor : ModuleRules
{
    public RustealEditor(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;

        PublicDependencyModuleNames.AddRange(
        [
            "Core",
            "CoreUObject",
            "Engine",
            "EditorSubsystem",
            "UnrealEd",
        ]);

        PrivateDependencyModuleNames.AddRange(
        [
            "Rusteal",
            "BlueprintGraph",
            "DirectoryWatcher",
            "MessageLog",
            "Slate",
            "SlateCore",
        ]);
    }
}
