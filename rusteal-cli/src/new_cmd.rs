// `rusteal new`: a UE project with Rust inside, built and ready to open.
//
// The UE side comes from the engine's own Blank C++ template. The editor's
// New Project dialog is a recipe every template ships in its
// `Config/TemplateDefs.ini` (copy, ignore, rename, replace, shared content);
// this reproduces it, then installs the Rusteal plugins, writes the Rust
// workspace and runs the build pipeline.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use rusteal_codegen::config::ProjectLayout;

use crate::{build_cmd, setup, templates};

/// The engine template a new project starts from: the C++ one, so that UBT has
/// a module and an Editor target to build the plugins with.
const TEMPLATE: &str = "TP_Blank";

/// Extensions the template recipe renames and rewrites inside.
const TEXT_EXTENSIONS: &[&str] = &["cpp", "h", "ini", "cs"];

/// glam version written into a new project's workspace.
const GLAM_VERSION: &str = "0.33";

pub struct NewOptions<'a> {
    pub name: &'a str,
    pub parent: &'a Path,
    pub engine: &'a Path,
    /// A Rusteal checkout to depend on by path instead of the published crates.
    pub runtime_path: Option<&'a Path>,
    pub build: bool,
}

pub fn run_new(opts: &NewOptions) {
    validate_name(opts.name);
    let root = opts.parent.join(opts.name);
    if root.exists() {
        eprintln!("Error: {} already exists.", root.display());
        std::process::exit(1);
    }

    eprintln!(
        "rusteal new: creating {} from the {TEMPLATE} template",
        root.display()
    );
    let template = Template::load(opts.engine, TEMPLATE);
    template.instantiate(opts.name, &root);
    write_uproject(&template, opts.name, opts.engine, &root);
    write_project_ini(&template, opts.name, &root);
    std::fs::write(root.join(".gitignore"), templates::raw("project.gitignore"))
        .unwrap_or_else(|e| panic!("Failed to write .gitignore: {e}"));

    eprintln!("rusteal new: installing the Rusteal plugins");
    setup::run_setup(&root, opts.engine);

    let crate_name = setup::default_crate_name(&root);
    eprintln!("rusteal new: writing Rust/ (crate {crate_name})");
    write_rust_workspace(&root, opts.name, &crate_name, opts.runtime_path);

    if opts.build {
        build_cmd::run_build(&root, opts.engine, None, 1);
    }

    eprintln!("\nrusteal new: done.");
    eprintln!("  cd {}", root.display());
    if !opts.build {
        eprintln!("  rusteal build      # UE build, bindings, plugin, cargo, deploy");
    }
    eprintln!(
        "  # open {}.uproject, drop a HelloActor into the level, press Play",
        opts.name
    );
}

/// UE project names: letters and digits, starting with a letter, 20 at most.
fn validate_name(name: &str) {
    let ok = name.len() <= 20
        && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && name.chars().all(|c| c.is_ascii_alphanumeric());
    if !ok {
        eprintln!(
            "Error: '{name}' is not a valid project name (letters and digits, \
             starting with a letter, 20 characters at most)."
        );
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// The engine template's own recipe (Config/TemplateDefs.ini)
// ---------------------------------------------------------------------------

struct Replacement {
    from: String,
    to: String,
    case_sensitive: bool,
}

struct Template {
    name: String,
    dir: PathBuf,
    engine: PathBuf,
    folders_to_ignore: Vec<String>,
    files_to_ignore: Vec<String>,
    folder_renames: Vec<(String, String)>,
    filename_replacements: Vec<Replacement>,
    content_replacements: Vec<Replacement>,
    /// (mount name, detail level) of the shared content packs to copy in.
    shared_packs: Vec<(String, String)>,
    /// Classes the template declares with its own prefix: the project needs a
    /// redirect for each, or its assets fail to load.
    prefixed_classes: Vec<String>,
}

impl Template {
    fn load(engine: &Path, name: &str) -> Self {
        let dir = engine.join("Templates").join(name);
        let defs_path = dir.join("Config/TemplateDefs.ini");
        let defs = std::fs::read_to_string(&defs_path).unwrap_or_else(|e| {
            eprintln!("Error: cannot read {}: {e}", defs_path.display());
            std::process::exit(1);
        });

        let mut t = Template {
            name: name.to_string(),
            dir: dir.clone(),
            engine: engine.to_path_buf(),
            folders_to_ignore: Vec::new(),
            files_to_ignore: Vec::new(),
            folder_renames: Vec::new(),
            filename_replacements: Vec::new(),
            content_replacements: Vec::new(),
            shared_packs: Vec::new(),
            prefixed_classes: Vec::new(),
        };

        for raw in defs.trim_start_matches('\u{feff}').lines() {
            let line = raw.trim();
            if line.starts_with(';') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            match key.trim() {
                "FoldersToIgnore" => t.folders_to_ignore.push(value.trim_matches('"').to_string()),
                "FilesToIgnore" => t.files_to_ignore.push(value.trim_matches('"').to_string()),
                "FolderRenames" => {
                    if let (Some(from), Some(to)) = (field(value, "From"), field(value, "To")) {
                        t.folder_renames.push((from, to));
                    }
                }
                key @ ("FilenameReplacements" | "ReplacementsInFiles") => {
                    if let (Some(from), Some(to)) = (field(value, "From"), field(value, "To")) {
                        let case_sensitive =
                            !value.to_lowercase().contains("bcasesensitive=false");
                        let rule = Replacement { from, to, case_sensitive };
                        if key == "FilenameReplacements" {
                            t.filename_replacements.push(rule);
                        } else {
                            t.content_replacements.push(rule);
                        }
                    }
                }
                "SharedContentPacks" => {
                    let mount = field(value, "MountName");
                    let level = value
                        .split("DetailLevels=(\"")
                        .nth(1)
                        .and_then(|rest| rest.split('"').next())
                        .map(str::to_string);
                    if let (Some(mount), Some(level)) = (mount, level) {
                        t.shared_packs.push((mount, level));
                    }
                }
                _ => {}
            }
        }

        if let Ok(entries) = std::fs::read_dir(dir.join("Source").join(name)) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "h") {
                    let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
                    if stem != name && stem.starts_with(name) {
                        t.prefixed_classes.push(stem);
                    }
                }
            }
        }
        t.prefixed_classes.sort();
        t
    }

    /// Expand the `%TEMPLATENAME%` / `%PROJECTNAME%` placeholders.
    fn expand(&self, text: &str, project: &str) -> String {
        text.replace("%TEMPLATENAME_UPPERCASE%", &self.name.to_uppercase())
            .replace("%TEMPLATENAME_LOWERCASE%", &self.name.to_lowercase())
            .replace("%TEMPLATENAME%", &self.name)
            .replace("%PROJECTNAME_UPPERCASE%", &project.to_uppercase())
            .replace("%PROJECTNAME_LOWERCASE%", &project.to_lowercase())
            .replace("%PROJECTNAME%", project)
    }

    fn apply(&self, rules: &[Replacement], text: &str, project: &str) -> String {
        let mut out = text.to_string();
        for rule in rules {
            let from = self.expand(&rule.from, project);
            let to = self.expand(&rule.to, project);
            out = if rule.case_sensitive {
                out.replace(&from, &to)
            } else {
                replace_case_insensitive(&out, &from, &to)
            };
        }
        out
    }

    /// Copy the template into `root` applying its ignores, renames and
    /// replacements, then add the shared content packs it asks for.
    fn instantiate(&self, project: &str, root: &Path) {
        let mut copied = 0usize;
        for path in walk(&self.dir) {
            let rel = path.strip_prefix(&self.dir).unwrap();
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            let rel_lower = rel_str.to_lowercase();

            let ignored_folder = self.folders_to_ignore.iter().any(|folder| {
                let folder = self.expand(folder, project).to_lowercase();
                rel_lower == folder || rel_lower.starts_with(&format!("{folder}/"))
            });
            let ignored_file = self
                .files_to_ignore
                .iter()
                .any(|file| self.expand(file, project).to_lowercase() == rel_lower);
            if ignored_folder || ignored_file {
                continue;
            }

            let mut out = rel_str.clone();
            for (from, to) in &self.folder_renames {
                let from = self.expand(from, project);
                if let Some(rest) = out.strip_prefix(&format!("{from}/")) {
                    out = format!("{}/{rest}", self.expand(to, project));
                }
            }
            let is_text = Path::new(&out).extension().is_some_and(|ext| {
                TEXT_EXTENSIONS.contains(&ext.to_string_lossy().to_lowercase().as_str())
            });
            if is_text {
                let (dir, name) = match out.rsplit_once('/') {
                    Some((dir, name)) => (format!("{dir}/"), name.to_string()),
                    None => (String::new(), out.clone()),
                };
                out = format!("{dir}{}", self.apply(&self.filename_replacements, &name, project));
            }

            let target = root.join(&out);
            std::fs::create_dir_all(target.parent().unwrap())
                .unwrap_or_else(|e| panic!("Failed to create {}: {e}", target.display()));
            if is_text {
                let text = std::fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("Failed to read {}: {e}", path.display()));
                let text = text.trim_start_matches('\u{feff}');
                std::fs::write(&target, self.apply(&self.content_replacements, text, project))
                    .unwrap_or_else(|e| panic!("Failed to write {}: {e}", target.display()));
            } else {
                std::fs::copy(&path, &target)
                    .unwrap_or_else(|e| panic!("Failed to copy {}: {e}", path.display()));
            }
            copied += 1;
        }
        eprintln!("  {copied} template files");

        for (mount, level) in &self.shared_packs {
            let content = self
                .engine
                .join("Templates/TemplateResources")
                .join(level)
                .join(mount)
                .join("Content");
            let copied = copy_dir(&content, &root.join("Content").join(mount));
            eprintln!("  shared pack {mount}: {copied} files");
        }
    }
}

/// `Key="value"` inside a TemplateDefs struct literal.
fn field(value: &str, key: &str) -> Option<String> {
    let start = value.find(&format!("{key}=\""))? + key.len() + 2;
    let end = value[start..].find('"')? + start;
    Some(value[start..end].to_string())
}

fn replace_case_insensitive(text: &str, from: &str, to: &str) -> String {
    if from.is_empty() {
        return text.to_string();
    }
    let lower = text.to_lowercase();
    let from_lower = from.to_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    let mut search = 0;
    while let Some(pos) = lower[search..].find(&from_lower) {
        let at = search + pos;
        out.push_str(&text[last..at]);
        out.push_str(to);
        last = at + from.len();
        search = last;
    }
    out.push_str(&text[last..]);
    out
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

fn copy_dir(from: &Path, to: &Path) -> usize {
    let mut copied = 0;
    for path in walk(from) {
        let target = to.join(path.strip_prefix(from).unwrap());
        std::fs::create_dir_all(target.parent().unwrap())
            .unwrap_or_else(|e| panic!("Failed to create {}: {e}", target.display()));
        std::fs::copy(&path, &target)
            .unwrap_or_else(|e| panic!("Failed to copy {}: {e}", path.display()));
        copied += 1;
    }
    copied
}

// ---------------------------------------------------------------------------
// .uproject and Config
// ---------------------------------------------------------------------------

fn write_uproject(template: &Template, project: &str, engine: &Path, root: &Path) {
    let source = template.dir.join(format!("{}.uproject", template.name));
    let text = std::fs::read_to_string(&source)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", source.display()));
    let mut json: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .unwrap_or_else(|e| panic!("Failed to parse {}: {e}", source.display()));

    json["EngineAssociation"] = serde_json::Value::String(engine_association(engine));
    if let Some(modules) = json.get_mut("Modules").and_then(|m| m.as_array_mut()) {
        for module in modules {
            if let Some(name) = module.get("Name").and_then(|n| n.as_str()) {
                module["Name"] = serde_json::Value::String(name.replace(&template.name, project));
            }
        }
    }

    let path = root.join(format!("{project}.uproject"));
    let mut text = serde_json::to_string_pretty(&json).expect("serializable uproject");
    text.push('\n');
    std::fs::write(&path, text)
        .unwrap_or_else(|e| panic!("Failed to write {}: {e}", path.display()));
}

/// "5.8" for an installed build, from Engine/Build/Build.version.
fn engine_association(engine: &Path) -> String {
    let path = engine.join("Engine/Build/Build.version");
    let version: Option<serde_json::Value> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok());
    match version {
        Some(v) => format!(
            "{}.{}",
            v["MajorVersion"].as_u64().unwrap_or(5),
            v["MinorVersion"].as_u64().unwrap_or(0)
        ),
        None => String::new(),
    }
}

/// What the editor adds to the template's ini files: a project id and name, and
/// the redirects from the template's module and classes to the project's.
fn write_project_ini(template: &Template, project: &str, root: &Path) {
    let game_ini = root.join("Config/DefaultGame.ini");
    let mut game = std::fs::read_to_string(&game_ini).unwrap_or_default();
    if !game.contains("ProjectID=") {
        if !game.is_empty() && !game.ends_with('\n') {
            game.push('\n');
        }
        game.push_str(&format!(
            "\n[/Script/EngineSettings.GeneralProjectSettings]\nProjectID={}\nProjectName={project}\n",
            project_id(project)
        ));
        std::fs::write(&game_ini, game)
            .unwrap_or_else(|e| panic!("Failed to write {}: {e}", game_ini.display()));
    }

    let mut lines = vec![
        String::new(),
        "[/Script/Engine.Engine]".to_string(),
        format!(
            "+ActiveGameNameRedirects=(OldGameName=\"{}\",NewGameName=\"/Script/{project}\")",
            template.name
        ),
        format!(
            "+ActiveGameNameRedirects=(OldGameName=\"/Script/{}\",NewGameName=\"/Script/{project}\")",
            template.name
        ),
    ];
    for class in &template.prefixed_classes {
        lines.push(format!(
            "+ActiveClassRedirects=(OldClassName=\"{class}\",NewClassName=\"{}\")",
            class.replace(&template.name, project)
        ));
    }

    let engine_ini = root.join("Config/DefaultEngine.ini");
    let mut text = std::fs::read_to_string(&engine_ini).unwrap_or_default();
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&lines.join("\n"));
    text.push('\n');
    std::fs::write(&engine_ini, text)
        .unwrap_or_else(|e| panic!("Failed to write {}: {e}", engine_ini.display()));
}

/// A 32-hex-digit id like the editor's: unique enough for a project file.
fn project_id(project: &str) -> String {
    let mut hasher = DefaultHasher::new();
    project.hash(&mut hasher);
    std::time::SystemTime::now().hash(&mut hasher);
    std::process::id().hash(&mut hasher);
    let high = hasher.finish();
    high.hash(&mut hasher);
    format!("{high:016X}{:016X}", hasher.finish())
}

// ---------------------------------------------------------------------------
// Rust workspace
// ---------------------------------------------------------------------------

fn write_rust_workspace(
    root: &Path,
    project: &str,
    crate_name: &str,
    runtime_path: Option<&Path>,
) {
    let layout = ProjectLayout::new(root);
    let rust = layout.rust_workspace();
    let game = rust.join(crate_name);
    std::fs::create_dir_all(game.join("src"))
        .unwrap_or_else(|e| panic!("Failed to create {}: {e}", game.display()));

    let mut ctx = tera::Context::new();
    ctx.insert("project", project);
    ctx.insert("crate_name", crate_name);
    // A generated project pins the published crates to the CLI's own version:
    // the generated bindings are tied to this runtime's FFI layout.
    ctx.insert("version", env!("CARGO_PKG_VERSION"));
    ctx.insert("glam_version", GLAM_VERSION);
    if let Some(path) = runtime_path {
        let checkout = path.canonicalize().unwrap_or_else(|e| {
            eprintln!("Error: --runtime-path {}: {e}", path.display());
            std::process::exit(1);
        });
        if !checkout.join("rusteal-runtime/Cargo.toml").exists() {
            eprintln!(
                "Error: {} is not a Rusteal checkout (no rusteal-runtime/Cargo.toml).",
                checkout.display()
            );
            std::process::exit(1);
        }
        ctx.insert("runtime_path", &checkout.to_string_lossy().replace('\\', "/"));
    }

    write(&rust.join("Cargo.toml"), &templates::render("workspace.Cargo.toml.tera", &ctx));
    write(&game.join("Cargo.toml"), &templates::render("game.Cargo.toml.tera", &ctx));
    write(&game.join("src/lib.rs"), &templates::render("lib.rs.tera", &ctx));
    write(&game.join("src/hello.rs"), &templates::render("hello.rs.tera", &ctx));

    // The bindings crate is written by the build; leave a placeholder so the
    // workspace resolves before the first `rusteal build`.
    let bindings = layout.bindings_crate();
    if !bindings.join("Cargo.toml").exists() {
        std::fs::create_dir_all(bindings.join("src"))
            .unwrap_or_else(|e| panic!("Failed to create {}: {e}", bindings.display()));
        write(
            &bindings.join("Cargo.toml"),
            "# Placeholder, replaced by the codegen step of `rusteal build`.\n\n\
             [package]\nname = \"bindings\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n\n\
             [dependencies]\nrusteal-core = { workspace = true }\nrusteal-ffi = { workspace = true }\n",
        );
        write(&bindings.join("src/lib.rs"), "// Written by `rusteal build`.\n");
    }
}

fn write(path: &Path, contents: &str) {
    std::fs::write(path, contents)
        .unwrap_or_else(|e| panic!("Failed to write {}: {e}", path.display()));
}
