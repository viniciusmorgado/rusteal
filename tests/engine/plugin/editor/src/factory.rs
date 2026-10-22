use bindings::core_ue::Object;
use bindings::engine::DataAsset;
use bindings::rusteal_editor::RustealFactory;
use bindings::unreal_ed::FactoryExt;
use rusteal_runtime::runtime::{RustealResult, SubclassOf, Text, UObjectRef};
use rusteal_runtime::{uclass, uclass_impl};

#[uclass(parent = DataAsset)]
pub struct ProbeNote {
    #[uproperty(EditAnywhere)]
    text: String,
}

#[uclass(parent = RustealFactory)]
pub struct ProbeNoteFactory {}

#[uclass_impl]
impl ProbeNoteFactory {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let factory = self.as_ref().checked()?;
        factory.set_supported_class(SubclassOf::<ProbeNote>::base().upcast_to::<Object>());
        factory.set_create_new(true);
        Ok(())
    }

    #[ufunction(Override)]
    fn receive_created_new(&mut self, new_object: UObjectRef<Object>, _context: UObjectRef<Object>) {
        if let Ok(mut note) = ProbeNote::from_obj(new_object) {
            note.set_text("made by Rust");
        }
    }

    #[ufunction(Override)]
    fn receive_get_display_name(&self) -> Text {
        Text::from("Rusteal probe note")
    }
}

#[uclass(parent = RustealFactory)]
pub struct ProbeNoteImporter {}

#[uclass_impl]
impl ProbeNoteImporter {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let factory = self.as_ref().checked()?;
        factory.set_supported_class(SubclassOf::<ProbeNote>::base().upcast_to::<Object>());
        factory.set_editor_import(true);
        factory
            .formats()
            .push(&"probe;Rusteal probe note".to_string())?;
        Ok(())
    }

    #[ufunction(Override)]
    fn receive_import_file(&mut self, new_object: UObjectRef<Object>, filename: String) -> bool {
        let Ok(contents) = std::fs::read_to_string(&filename) else {
            return false;
        };

        match ProbeNote::from_obj(new_object) {
            Ok(mut note) => {
                note.set_text(contents.trim());
                true
            }
            Err(_) => false,
        }
    }

    #[ufunction(Override)]
    fn receive_can_import(&self, filename: String) -> bool {
        filename.ends_with(".probe")
    }

}
