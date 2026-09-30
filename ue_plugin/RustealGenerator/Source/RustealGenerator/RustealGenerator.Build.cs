namespace UnrealBuildTool.Rules;

public class RustealGenerator : ModuleRules
{
    public RustealGenerator(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;

        PublicDependencyModuleNames.AddRange(["Core", "CoreUObject", "Engine"]);
    }
}
