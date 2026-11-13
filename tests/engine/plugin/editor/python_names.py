import os

import unreal

target = unreal.new_object(unreal.load_class(None, "/Script/RustealProbe.ProbeRecord"))

expected = [
    ("get_editor_property('Charge')", True, lambda: target.get_editor_property("Charge")),
    ("get_editor_property('charge')", True, lambda: target.get_editor_property("charge")),
    ("set_editor_property('charge', 4)", True, lambda: target.set_editor_property("charge", 4)),
    ("call_method('AddCharge', (1,))", True, lambda: target.call_method("AddCharge", (1,))),
    ("the attribute .charge", False, lambda: target.charge),
    ("call_method('add_charge', (1,))", False, lambda: target.call_method("add_charge", (1,))),
]

surprises = []

for what, works, attempt in expected:
    try:
        attempt()
        worked = True
    except Exception:
        worked = False

    if worked != works:
        surprises.append(f"{what} {'works' if worked else 'fails'}")

name = "editor_python_reaches_rust_properties_and_functions"
line = f"PASS {name}" if not surprises else f"FAIL {name}: {', '.join(surprises)}"

with open(os.environ["RUSTEAL_TEST_RESULTS"], "a") as results:
    results.write(line + "\n")
