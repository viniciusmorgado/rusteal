namespace UnrealBuildTool.Rules;

// Editor-only parents for Rust classes, the editor's own extension points that
// announce themselves through C++ virtuals; and the editor side of hot reload.
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

        // Rusteal: the libraries it hosts; UnrealEd: FReload, which reinstances
        // the Rust classes a hot reload replaced; DirectoryWatcher and Slate:
        // reloading a library when it is deployed again, with a notification.
        PrivateDependencyModuleNames.AddRange(
        [
            "Rusteal",
            "UnrealEd",
            "DirectoryWatcher",
            "Slate",
            "SlateCore",
        ]);
    }
}
