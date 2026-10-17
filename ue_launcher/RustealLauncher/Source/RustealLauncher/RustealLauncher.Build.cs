namespace UnrealBuildTool.Rules;

public class RustealLauncher : ModuleRules
{
    public RustealLauncher(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;

        PrivateDependencyModuleNames.AddRange(
        [
            "Core",
            "CoreUObject",
            "Engine",
            "InputCore",
            "Json",
            "MainFrame",
            "Projects",
            "Slate",
            "SlateCore",
            "UnrealEd",
        ]);
    }
}
