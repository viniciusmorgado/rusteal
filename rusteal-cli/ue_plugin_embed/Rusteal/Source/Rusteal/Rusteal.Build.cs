using UnrealBuildTool;
using System.IO;

public class Rusteal : ModuleRules
{
    public Rusteal(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;

        // Modules the hand-written runtime needs regardless of which bindings are generated
        // (RustealWidgetApiImpl.cpp uses UMG; UMG needs Slate/SlateCore/InputCore).
        PublicDependencyModuleNames.AddRange(new string[]
        {
            "Core",
            "CoreUObject",
            "Engine",
            "InputCore",
            "SlateCore",
            "Slate",
            "UMG",
        });

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
}
