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
        ]);

        PrivateDependencyModuleNames.AddRange(
        [
            "Rusteal",
            "UnrealEd",
            "BlueprintGraph",
            "DirectoryWatcher",
            "Slate",
            "SlateCore",
        ]);
    }
}
