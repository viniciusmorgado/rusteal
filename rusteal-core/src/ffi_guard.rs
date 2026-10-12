use std::cell::RefCell;

use rusteal_ffi::{FWeakObjectHandle, RustealErrorReport};

use crate::api::is_api_initialized;

struct PanicSite {
    file: String,
    line: u32,
    column: u32,
}

thread_local! {
    static PANIC_SITE: RefCell<Option<PanicSite>> = const { RefCell::new(None) };
}

pub fn install_panic_hook() {
    static INSTALLED: std::sync::Once = std::sync::Once::new();

    INSTALLED.call_once(|| {
        let previous = std::panic::take_hook();

        std::panic::set_hook(Box::new(move |info| {
            let site = info.location().map(|location| PanicSite {
                file: location.file().to_owned(),
                line: location.line(),
                column: location.column(),
            });

            PANIC_SITE.with(|slot| *slot.borrow_mut() = site);
            previous(info);
        }));
    });
}

pub fn ffi_boundary<F, R>(default: R, f: F) -> R
where
    F: FnOnce() -> R + std::panic::UnwindSafe,
{
    match std::panic::catch_unwind(f) {
        Ok(value) => value,
        Err(payload) => {
            if is_api_initialized() {
                report_panic(&panic_message(&payload));
            }

            default
        }
    }
}

fn report_panic(message: &str) {
    let site = PANIC_SITE.with(|slot| slot.borrow_mut().take());
    let summary = format!("Rust panic: {message}");

    let (details, file, line, column) = match &site {
        Some(site) => {
            let file = crate::error::absolute_path(&site.file);

            (
                format!(
                    "{summary}\n    at {}:{}:{}",
                    site.file, site.line, site.column
                ),
                file,
                site.line,
                site.column,
            )
        }
        None => (summary.clone(), String::new(), 0, 0),
    };

    let report = RustealErrorReport {
        level: crate::logging::LOG_ERROR,
        script_error: true,
        summary: summary.as_ptr(),
        summary_len: summary.len() as u32,
        details: details.as_ptr(),
        details_len: details.len() as u32,
        object: FWeakObjectHandle {
            object_index: -1,
            object_serial_number: 0,
        },
        file: file.as_ptr(),
        file_len: file.len() as u32,
        line,
        column,
    };

    unsafe {
        crate::ffi_dispatch::logging_report(&report);
    }
}

fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_owned()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown payload".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_boundary_returns_value_on_success() {
        let result = ffi_boundary(0i32, || 42);
        assert_eq!(result, 42);
    }

    #[test]
    fn ffi_boundary_returns_default_on_panic() {
        let result = ffi_boundary(-1i32, || {
            panic!("test panic");
        });

        assert_eq!(result, -1);
    }

    #[test]
    fn ffi_boundary_returns_default_on_string_panic() {
        let result = ffi_boundary(false, || -> bool {
            panic!("{}", "formatted panic");
        });

        assert!(!result);
    }

    #[test]
    fn panic_messages_keep_their_text() {
        let payload: Box<dyn std::any::Any + Send> = Box::new("boom");
        assert_eq!(panic_message(&payload), "boom");
        let payload: Box<dyn std::any::Any + Send> = Box::new(String::from("bang"));
        assert_eq!(panic_message(&payload), "bang");
    }
}
