use bindings::engine::KismetMathLibrary;
use rusteal_runtime::runtime::automation::{self, Contexts};

pub const FAILURE: &str = "the failure this test reports on purpose";

pub fn register() {
    let passes = automation::register_test("Rusteal.Tests.Passes", Contexts::ALL, || {
        automation::add_info("a Rust automation test ran");

        let sum = KismetMathLibrary::add_int_int(2, Some(3));

        if sum == 5 {
            Ok(())
        } else {
            Err(format!("2 + 3 is {sum}"))
        }
    });

    let fails = automation::register_test(
        "Rusteal.Tests.FailsOnPurpose",
        Contexts::EDITOR,
        || Err(FAILURE),
    );

    for registered in [passes, fails] {
        if let Err(error) = registered {
            log::error!("[RustealTests] cannot register an automation test: {error:?}");
        }
    }
}
