namespace UnrealBuildTool.Rules;

// Editor-only parents for Rust classes: the editor's own extension points that
// announce themselves through C++ virtuals.
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
    }
}
