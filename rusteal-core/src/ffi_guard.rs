use crate::api::is_api_initialized;

pub fn ffi_boundary<F, R>(default: R, f: F) -> R
where
    F: FnOnce() -> R + std::panic::UnwindSafe,
{
    match std::panic::catch_unwind(f) {
        Ok(value) => value,
        Err(payload) => {
            if is_api_initialized() {
                let msg = panic_message(&payload);
                let bytes = msg.as_bytes();

                unsafe {
                    crate::ffi_dispatch::logging_log(2, bytes.as_ptr(), bytes.len() as u32);
                }
            }

            default
        }
    }
}

fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        format!("[Rusteal] Rust panic: {s}")
    } else if let Some(s) = payload.downcast_ref::<String>() {
        format!("[Rusteal] Rust panic: {s}")
    } else {
        "[Rusteal] Rust panic (unknown payload)".to_string()
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
}
