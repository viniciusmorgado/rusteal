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
        PrivateDependencyModuleNames.AddRange(["HTTP", "AIModule"]);

        string StaticSwitch = StaticSwitchFile(Target);

        if (File.Exists(StaticSwitch))
        {
            ExternalDependencies.Add(StaticSwitch);
        }

        if (LinksRustStatically(Target))
        {
            PrivateDependencyModuleNames.Add("RustealStatic");
            PublicDefinitions.Add("RUSTEAL_STATIC=1");
        }
        else
        {
            PublicDefinitions.Add("RUSTEAL_STATIC=0");
            AddRustLibrary(this, Target, PluginDirectory, "rusteal");
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

    static string StaticSwitchFile(ReadOnlyTargetRules Target)
    {
        string Project = Target.ProjectFile != null ? Target.ProjectFile.Directory.FullName : "";

        return Path.Combine(Project, "Plugins", "Rusteal", "Source", "Rusteal", "Generated",
            "static_link.txt");
    }

    public static bool LinksRustStatically(ReadOnlyTargetRules Target)
    {
        if (Target.Type == TargetType.Editor)
        {
            return false;
        }

        string StaticSwitch = StaticSwitchFile(Target);

        return File.Exists(StaticSwitch) && File.ReadAllText(StaticSwitch).Trim() == "1";
    }

    public static void AddRustLibrary(ModuleRules Rules, ReadOnlyTargetRules Target,
        string PluginDirectory, string Stem)
    {
        if (LinksRustStatically(Target))
        {
            return;
        }

        string FileName = RustealLibraryFileName(Target, Stem);

        if (FileName == null)
        {
            return;
        }

        string Library = Path.Combine(PluginDirectory, "Binaries", Target.Platform.ToString(),
            FileName);

        if (File.Exists(Library))
        {
            Rules.RuntimeDependencies.Add(Library);
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

        if (Target.Platform == UnrealTargetPlatform.Linux || Target.Platform == UnrealTargetPlatform.LinuxArm64)
        {
            return "lib" + Stem + ".so";
        }

        return null;
    }
}
