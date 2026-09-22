// The files `rusteal setup` and `rusteal new` write into a project, embedded
// at build time from `templates/`.
//
// `*.tera` files are rendered with the given context; everything else is
// written verbatim (the project's `.gitignore`, which has nothing to fill in).

include!(concat!(env!("OUT_DIR"), "/template_files.rs"));

/// A template's raw text.
pub fn raw(name: &str) -> &'static str {
    TEMPLATE_FILES
        .iter()
        .find(|(file, _)| *file == name)
        .map(|(_, contents)| *contents)
        .unwrap_or_else(|| panic!("template {name} is not embedded"))
}

/// Render a Tera template.
pub fn render(name: &str, context: &tera::Context) -> String {
    tera::Tera::one_off(raw(name), context, false)
        .unwrap_or_else(|e| panic!("Failed to render {name}: {e}"))
}
