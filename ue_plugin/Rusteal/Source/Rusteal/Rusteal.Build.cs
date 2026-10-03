using System.IO;

namespace UnrealBuildTool.Rules;

public class Rusteal : ModuleRules
{
    public Rusteal(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;

        // Modules the hand-written runtime needs regardless of which bindings are generated
        // (RustealWidgetApiImpl.cpp uses UMG; UMG needs Slate/SlateCore/InputCore;
        // RustealInputApiImpl.cpp uses EnhancedInput, which Rusteal.uplugin enables).
        PublicDependencyModuleNames.AddRange(
        [
            "Core",
            "CoreUObject",
            "Engine",
            "InputCore",
            "SlateCore",
            "Slate",
            "UMG",
            "EnhancedInput",
        ]);

        // IPluginManager: the plugin reads its own version from its descriptor.
        PrivateDependencyModuleNames.Add("Projects");

        // The game's Rust library, staged with a packaged game: `rusteal build`
        // deploys it into the plugin's binaries (RustealModule.cpp loads it).
        string Library = Path.Combine(PluginDirectory, "Binaries", Target.Platform.ToString(),
            RustealLibraryFileName(Target, "rusteal"));
        if (File.Exists(Library))
        {
            RuntimeDependencies.Add(Library);
        }

        // Modules the generated bindings call into, listed by rusteal-codegen in
        // Generated/module_deps.txt. Absent before the first codegen run, which is fine:
        // the generated wrappers do not exist yet either.
        string DepsFile = Path.Combine(ModuleDirectory, "Generated", "module_deps.txt");
        if (File.Exists(DepsFile))
        {
            foreach (string Module in File.ReadAllLines(DepsFile))
            {
                string Trimmed = Module.Trim();
                if (Trimmed.Length > 0 && !PublicDependencyModuleNames.Contains(Trimmed))
                {
                    PublicDependencyModuleNames.Add(Trimmed);
                }
            }
        }
    }

    /// <summary>The platform's file name for a Rust library: librusteal.so,
    /// rusteal.dll, librusteal.dylib for "rusteal".</summary>
    public static string RustealLibraryFileName(ReadOnlyTargetRules Target, string Stem)
    {
        if (Target.Platform == UnrealTargetPlatform.Win64)
        {
            return Stem + ".dll";
        }
        if (Target.Platform == UnrealTargetPlatform.Mac)
        {
            return "lib" + Stem + ".dylib";
        }
        return "lib" + Stem + ".so";
    }
}