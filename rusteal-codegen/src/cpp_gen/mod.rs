pub mod fill_table;
pub mod func_ids;
pub mod wrapper;

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use crate::context::CodegenContext;
use crate::write_if_changed;

pub fn generate(ctx: &CodegenContext, out_dir: &Path) {
    std::fs::create_dir_all(out_dir).expect("Failed to create C++ output directory");

    let mut by_class: BTreeMap<(String, String), Vec<&crate::context::FuncEntry>> = BTreeMap::new();

    for entry in &ctx.func_table {
        by_class
            .entry((entry.module_name.clone(), entry.class_name.clone()))
            .or_default()
            .push(entry);
    }

    let mut written: HashSet<String> = HashSet::new();

    for ((module, class), entries) in &by_class {
        let code = wrapper::generate_wrapper_file(entries, ctx);
        let filename = format!("RustealFunc_{}_{}.cpp", module, class);

        write_if_changed(&out_dir.join(&filename), &code)
            .unwrap_or_else(|e| panic!("Failed to write {filename}: {e}"));

        written.insert(filename);
    }

    remove_stale_wrappers(out_dir, &written);

    let ids_code = func_ids::generate_cpp_func_ids(&ctx.func_table);

    write_if_changed(&out_dir.join("RustealFuncIds.h"), &ids_code)
        .expect("Failed to write RustealFuncIds.h");

    let fill_code = fill_table::generate_fill_table(&ctx.cpp_prefix, &ctx.func_table, &by_class);

    write_if_changed(&out_dir.join("RustealFillFuncTable.cpp"), &fill_code)
        .expect("Failed to write RustealFillFuncTable.cpp");
}

fn remove_stale_wrappers(out_dir: &Path, written: &HashSet<String>) {
    let Ok(entries) = std::fs::read_dir(out_dir) else {
        return;
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if name.starts_with("RustealFunc_")
            && name.ends_with(".cpp")
            && !written.contains(name.as_ref())
        {
            std::fs::remove_file(entry.path())
                .unwrap_or_else(|e| panic!("Failed to remove stale {name}: {e}"));
        }
    }
}
