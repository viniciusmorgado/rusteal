use std::path::Path;

use rusteal_codegen::config::{PluginLayout, find_uproject};

use crate::project_version::{self, Pins};
use crate::{build_cmd, new_cmd, setup, templates};

const RESERVED: &[&str] = &["Rusteal", "RustealGenerator", "RustealEditor"];

pub struct PluginNewOptions<'a> {
    pub name: &'a str,
    pub template: &'a str,
    pub root: &'a Path,
    pub engine: &'a Path,
    pub build: bool,
}

pub fn run_plugin_new(opts: &PluginNewOptions) {
    let next_step = create_plugin(opts.name, opts.template, opts.root, opts.engine);

    if opts.build {
        build_cmd::run_build(opts.root, opts.engine, None, 1, false, None);
    }

    eprintln!("\nrusteal plugin new: done.");

    if !opts.build {
        eprintln!("  rusteal build --all   # UE build, bindings, plugin, cargo, deploy");
    }

    eprintln!("  # {next_step}");
}

pub fn create_plugin(name: &str, template: &str, root: &Path, engine: &Path) -> String {
    validate_name(name, root);

    let (template, manifest) = templates::select_plugin(template).unwrap_or_else(|message| {
        eprint!("Error: {message}");
        std::process::exit(1);
    });

    if let Some(feature) = &manifest.feature
        && feature.plugin != name
        && templates::plugin_has_content(template)
    {
        fail(&format!(
            "the {template} feature's assets live under /{plugin}/, so its plugin is named \
             {plugin}: rusteal plugin new {plugin} --template {template}",
            plugin = feature.plugin
        ));
    }

    let dir = root.join("Plugins").join(name);

    if dir.exists() {
        fail(&format!("{} already exists.", dir.display()));
    }

    let layout = PluginLayout::new(&dir);

    eprintln!(
        "rusteal plugin new: creating {} from the {template} plugin template",
        dir.display()
    );

    let crate_name = setup::crate_name_for(name);
    let context = plugin_context(name, &crate_name, root);
    let written = templates::write_plugin_files(template, &dir, &context);
    eprintln!("rusteal plugin new: {written} files (crate {crate_name})");
    new_cmd::write_bindings_placeholder(&layout.rust_workspace().join("bindings"), &crate_name);

    setup::write_cpp_stubs(
        &layout.output(Default::default(), &crate_name).cpp_generated,
        &format!("{name}_"),
    );

    enable_in_uproject(root, name);

    if let Some(feature) = &manifest.feature {
        new_cmd::copy_shared_packs(engine, root, &feature.shared_packs);
    }

    tera::Tera::one_off(&manifest.next_step, &context, false)
        .unwrap_or_else(|e| panic!("Failed to render next_step: {e}"))
}

fn validate_name(name: &str, root: &Path) {
    let ok = (1..=32).contains(&name.len())
        && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && name.chars().all(|c| c.is_ascii_alphanumeric());

    if !ok {
        fail(&format!(
            "'{name}' is not a valid plugin name (letters and digits, starting with a letter, \
             32 characters at most)."
        ));
    }

    let project = find_uproject(root)
        .and_then(|path| path.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_default();

    if RESERVED.iter().any(|r| r.eq_ignore_ascii_case(name)) || project.eq_ignore_ascii_case(name) {
        fail(&format!(
            "'{name}' is taken: the plugin's module would clash with Rusteal's or the project's."
        ));
    }
}

fn plugin_context(plugin: &str, crate_name: &str, root: &Path) -> tera::Context {
    let mut ctx = tera::Context::new();
    ctx.insert("plugin", plugin);
    ctx.insert("crate_name", crate_name);
    ctx.insert("version", env!("CARGO_PKG_VERSION"));
    ctx.insert("glam_version", new_cmd::GLAM_VERSION);

    if let Ok(Pins::Checkout(checkout)) = project_version::read_pins(root) {
        let checkout = checkout.canonicalize().unwrap_or(checkout);

        ctx.insert(
            "runtime_path",
            &checkout.to_string_lossy().replace('\\', "/"),
        );
    }

    ctx
}

fn enable_in_uproject(root: &Path, plugin: &str) {
    let Some(path) = find_uproject(root) else {
        fail(&format!("no .uproject in {}", root.display()));
    };

    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", path.display())));

    let mut json: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .unwrap_or_else(|e| fail(&format!("cannot parse {}: {e}", path.display())));

    let plugins = json
        .as_object_mut()
        .unwrap_or_else(|| fail(&format!("{} is not a JSON object", path.display())))
        .entry("Plugins")
        .or_insert_with(|| serde_json::Value::Array(Vec::new()))
        .as_array_mut()
        .unwrap_or_else(|| fail(&format!("{}: Plugins is not an array", path.display())));

    if !plugins.iter().any(|p| p["Name"] == plugin) {
        plugins.push(serde_json::json!({ "Name": plugin, "Enabled": true }));
    }

    let mut out = serde_json::to_string_pretty(&json).expect("serializable uproject");
    out.push('\n');

    std::fs::write(&path, out)
        .unwrap_or_else(|e| fail(&format!("cannot write {}: {e}", path.display())));

    eprintln!("  {} lists {plugin}", path.display());
}

fn fail(message: &str) -> ! {
    eprintln!("Error: {message}");
    std::process::exit(1);
}
