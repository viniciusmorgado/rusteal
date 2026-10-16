using System.IO;
using UnrealBuildTool;

public class RustealStatic : ModuleRules
{
    public RustealStatic(ReadOnlyTargetRules Target) : base(Target)
    {
        Type = ModuleType.External;

        string Directory = Path.Combine(ModuleDirectory, "lib", Target.Platform.ToString());
        string Link = Path.Combine(Directory, "link.txt");

        if (!File.Exists(Link))
        {
            throw new BuildException(
                "Rusteal: no static Rust library for " + Target.Platform + " in " + Directory +
                ". Package with `rusteal package --static`.");
        }

        ExternalDependencies.Add(Link);

        foreach (string Line in File.ReadAllLines(Link))
        {
            string Entry = Line.Trim();

            if (Entry.StartsWith("archive="))
            {
                string Archive = Path.Combine(Directory, Entry.Substring("archive=".Length));
                PublicAdditionalLibraries.Add(Archive);
                ExternalDependencies.Add(Archive);
            }
            else if (Entry.StartsWith("system="))
            {
                PublicSystemLibraries.Add(Entry.Substring("system=".Length));
            }
        }
    }
}
