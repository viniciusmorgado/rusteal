using System.IO;

namespace UnrealBuildTool.Rules;

public class Rusteal : ModuleRules
{
    public Rusteal(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;

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

        PrivateDependencyModuleNames.Add("Projects");

        string Library = Path.Combine(PluginDirectory, "Binaries", Target.Platform.ToString(),
            RustealLibraryFileName(Target, "rusteal"));

        if (File.Exists(Library))
        {
            RuntimeDependencies.Add(Library);
        }

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
