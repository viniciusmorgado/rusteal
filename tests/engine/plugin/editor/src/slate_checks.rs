use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

use rusteal_runtime::runtime::slate::{
    self, Border, Button, CheckBox, EditableTextBox, HorizontalBox, Slot, SpinBox, TextBlock,
    VerticalBox,
};

fn ensure(condition: bool, what: impl Into<String>) -> Result<(), String> {
    if condition { Ok(()) } else { Err(what.into()) }
}

pub fn widgets_call_their_closures() -> Result<(), String> {
    let e = |e| format!("{e:?}");
    let callbacks_before = slate::live_callbacks();
    let widgets_before = slate::live_widgets();

    {
        let clicks = Arc::new(AtomicI32::new(0));
        let counter = clicks.clone();
        let shown = clicks.clone();

        let button = Button::new()
            .text("Add")
            .on_clicked(move || {
                counter.fetch_add(1, Ordering::SeqCst);
            })
            .build()
            .map_err(e)?;
        let label = TextBlock::new(move || format!("{} clicks", shown.load(Ordering::SeqCst)))
            .build()
            .map_err(e)?;

        button.simulate_click();
        button.simulate_click();
        let text = label.text().map_err(e)?;

        let field = EditableTextBox::new("start")
            .hint("a name")
            .build()
            .map_err(e)?;
        let field_text = field.text().map_err(e)?;

        let panel = VerticalBox::new()
            .slot(TextBlock::new("Probe"))
            .slot_with(
                Slot::default().padding(4.0).fill(1.0),
                HorizontalBox::new()
                    .slot(CheckBox::new(true).on_changed(|_| {}))
                    .slot(SpinBox::new(0.5, 0.0, 1.0).on_changed(|_| {})),
            )
            .slot(Border::new(TextBlock::new("framed")).padding(2.0))
            .slot(slate::spacer(8.0, 8.0))
            .build();

        ensure(
            clicks.load(Ordering::SeqCst) == 2 && text == "2 clicks",
            format!(
                "two clicks counted {}, the label says {text:?}",
                clicks.load(Ordering::SeqCst)
            ),
        )?;
        ensure(field_text == "start", format!("the text box says {field_text:?}"))?;
        ensure(panel.is_ok(), format!("the panel was not built: {panel:?}"))?;
    }

    let callbacks_after = slate::live_callbacks();
    let widgets_after = slate::live_widgets();

    ensure(
        callbacks_after == callbacks_before && widgets_after == widgets_before,
        format!(
            "{callbacks_before} callbacks and {widgets_before} widgets before, \
             {callbacks_after} and {widgets_after} after the widgets were dropped"
        ),
    )
}

pub fn tab_opens_with_rust_content() -> Result<(), String> {
    let built = Arc::new(AtomicI32::new(0));
    let counter = built.clone();

    slate::register_tab("RustealProbeTab", "Rusteal probe", move || {
        counter.fetch_add(1, Ordering::SeqCst);
        VerticalBox::new().slot(TextBlock::new("probe")).build()
    })
    .map_err(|e| format!("{e:?}"))?;

    let opened = slate::open_tab("RustealProbeTab");
    let times = built.load(Ordering::SeqCst);
    slate::unregister_tab("RustealProbeTab");

    ensure(
        opened && times == 1,
        format!("the tab opened: {opened}, its content was built {times} times"),
    )
}

pub fn details_panel_runs_a_rust_customization() -> Result<(), String> {
    use bindings::editor_subsystem::EditorSubsystem;
    use bindings::manual::world_ext::new_object;
    use bindings::unreal_ed::EditorSubsystemBlueprintLibrary;
    use rusteal_runtime::runtime::{SubclassOf, details};

    use crate::factory::ProbeNote;
    use crate::subsystem::ProbeEditorSubsystem;

    let e = |e| format!("{e:?}");
    let runs = Arc::new(AtomicI32::new(0));
    let seen = Arc::new(AtomicI32::new(0));
    let (counter, objects) = (runs.clone(), seen.clone());

    details::customize_class::<ProbeNote>(move |layout| {
        counter.fetch_add(1, Ordering::SeqCst);
        objects.store(layout.objects::<ProbeNote>().len() as i32, Ordering::SeqCst);
        layout.hide_property("Text")?;
        layout.add_row(
            "Rusteal",
            TextBlock::new("Made in"),
            Button::new().text("Rust"),
        )
    })
    .map_err(e)?;

    let outer = EditorSubsystemBlueprintLibrary::get_editor_subsystem(
        SubclassOf::<ProbeEditorSubsystem>::base().upcast_to::<EditorSubsystem>(),
    );
    let note = new_object::<ProbeNote>(&outer).map_err(e)?;

    let view = details::view(&note).map_err(e);
    details::remove_customization::<ProbeNote>();
    let _view = view?;

    let runs = runs.load(Ordering::SeqCst);
    let seen = seen.load(Ordering::SeqCst);

    ensure(
        runs >= 1 && seen == 1,
        format!("the customization ran {runs} times and saw {seen} objects"),
    )
}
