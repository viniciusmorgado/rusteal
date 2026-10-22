import os
import tempfile

import unreal

problems = []
tools = unreal.AssetToolsHelpers.get_asset_tools()
factory_class = unreal.load_class(None, "/Script/RustealProbeEditor.ProbeNoteFactory")
note_class = unreal.load_class(None, "/Script/RustealProbeEditor.ProbeNote")

created = tools.create_asset("RustealNote", "/Game/RustealTests", note_class, unreal.new_object(factory_class))

if created is None:
    problems.append("create_asset gave nothing")
elif created.get_editor_property("text") != "made by Rust":
    problems.append(f"the new note says {created.get_editor_property('text')!r}")

source = os.path.join(tempfile.mkdtemp(), "Imported.probe")

with open(source, "w") as file:
    file.write("parsed by Rust\n")

task = unreal.AssetImportTask()
task.set_editor_property("filename", source)
task.set_editor_property("destination_path", "/Game/RustealTests")
task.set_editor_property("automated", True)
task.set_editor_property("save", False)
task.set_editor_property(
    "factory",
    unreal.new_object(unreal.load_class(None, "/Script/RustealProbeEditor.ProbeNoteImporter")),
)
tools.import_asset_tasks([task])

imported = task.get_objects()

if not imported:
    problems.append("the import made nothing")
elif imported[0].get_editor_property("text") != "parsed by Rust":
    problems.append(f"the imported note says {imported[0].get_editor_property('text')!r}")

name = "editor_factory_creates_and_imports_assets"
line = f"PASS {name}" if not problems else f"FAIL {name}: {'; '.join(problems)}"

with open(os.environ["RUSTEAL_TEST_RESULTS"], "a") as results:
    results.write(line + "\n")
