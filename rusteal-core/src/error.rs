// Error types for the Rusteal runtime.

use std::fmt;

use rusteal_ffi::RustealErrorCode;

/// Rich error type for Rusteal operations.
#[derive(Debug)]
pub enum RustealError {
    ObjectDestroyed,
    InvalidCast,
    PropertyNotFound(String),
    FunctionNotFound(String),
    TypeMismatch,
    NullArgument,
    IndexOutOfRange,
    InvalidOperation(String),
    Internal(String),
    BufferTooSmall,
}

impl fmt::Display for RustealError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RustealError::ObjectDestroyed => write!(f, "object has been destroyed"),
            RustealError::InvalidCast => write!(f, "invalid cast"),
            RustealError::PropertyNotFound(name) => write!(f, "property not found: {name}"),
            RustealError::FunctionNotFound(name) => write!(f, "function not found: {name}"),
            RustealError::TypeMismatch => write!(f, "type mismatch"),
            RustealError::NullArgument => write!(f, "null argument"),
            RustealError::IndexOutOfRange => write!(f, "index out of range"),
            RustealError::InvalidOperation(msg) => write!(f, "invalid operation: {msg}"),
            RustealError::Internal(msg) => write!(f, "internal error: {msg}"),
            RustealError::BufferTooSmall => write!(f, "buffer too small"),
        }
    }
}

impl std::error::Error for RustealError {}

/// Convenience alias used throughout the runtime and generated code.
pub type RustealResult<T> = Result<T, RustealError>;

/// Convert an FFI error code to a `RustealResult<()>`.
/// `Ok` maps to `Ok(())`, all others map to the corresponding `RustealError`.
pub fn check_ffi(code: RustealErrorCode) -> RustealResult<()> {
    match code {
        RustealErrorCode::Ok => Ok(()),
        other => Err(RustealError::from(other)),
    }
}

/// Like `check_ffi`, but enriches property/function errors with the given name.
pub fn check_ffi_ctx(code: RustealErrorCode, context: &str) -> RustealResult<()> {
    match code {
        RustealErrorCode::Ok => Ok(()),
        RustealErrorCode::PropertyNotFound => Err(RustealError::PropertyNotFound(context.into())),
        RustealErrorCode::FunctionNotFound => Err(RustealError::FunctionNotFound(context.into())),
        RustealErrorCode::InvalidOperation => Err(RustealError::InvalidOperation(context.into())),
        other => Err(RustealError::from(other)),
    }
}

/// Assert that an FFI call returned `Ok`. Used for codegen-generated methods
/// where handle validation has already been performed and the C++ wrapper
/// is expected to always succeed. Panics in debug builds if the code is not `Ok`.
#[inline(always)]
pub fn ffi_infallible(code: RustealErrorCode) {
    debug_assert_eq!(
        code,
        RustealErrorCode::Ok,
        "FFI call returned {:?} after pre-validation",
        code
    );
}

/// Like [`ffi_infallible`], but includes a context string in the panic message.
#[inline(always)]
pub fn ffi_infallible_ctx(code: RustealErrorCode, ctx: &str) {
    debug_assert_eq!(
        code,
        RustealErrorCode::Ok,
        "FFI '{}' returned {:?} after pre-validation",
        ctx,
        code
    );
}

impl From<RustealErrorCode> for RustealError {
    #[allow(clippy::match_same_arms)]
    fn from(code: RustealErrorCode) -> Self {
        match code {
            RustealErrorCode::Ok => {
                // Callers should not convert Ok into an error. If they do,
                // treat it as an internal logic bug.
                RustealError::Internal("unexpected Ok error code".into())
            }
            RustealErrorCode::ObjectDestroyed => RustealError::ObjectDestroyed,
            RustealErrorCode::InvalidCast => RustealError::InvalidCast,
            RustealErrorCode::PropertyNotFound => RustealError::PropertyNotFound(String::new()),
            RustealErrorCode::FunctionNotFound => RustealError::FunctionNotFound(String::new()),
            RustealErrorCode::TypeMismatch => RustealError::TypeMismatch,
            RustealErrorCode::NullArgument => RustealError::NullArgument,
            RustealErrorCode::IndexOutOfRange => RustealError::IndexOutOfRange,
            RustealErrorCode::InvalidOperation => RustealError::InvalidOperation(String::new()),
            RustealErrorCode::InternalError => RustealError::Internal(String::new()),
            RustealErrorCode::BufferTooSmall => RustealError::BufferTooSmall,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_ffi_ok_returns_ok() {
        assert!(check_ffi(RustealErrorCode::Ok).is_ok());
    }

    #[test]
    fn check_ffi_errors_map_correctly() {
        let cases = [
            (RustealErrorCode::ObjectDestroyed, "ObjectDestroyed"),
            (RustealErrorCode::InvalidCast, "InvalidCast"),
            (RustealErrorCode::PropertyNotFound, "PropertyNotFound"),
            (RustealErrorCode::FunctionNotFound, "FunctionNotFound"),
            (RustealErrorCode::TypeMismatch, "TypeMismatch"),
            (RustealErrorCode::NullArgument, "NullArgument"),
            (RustealErrorCode::IndexOutOfRange, "IndexOutOfRange"),
            (RustealErrorCode::InvalidOperation, "InvalidOperation"),
            (RustealErrorCode::InternalError, "Internal"),
        ];
        for (code, expected_variant) in cases {
            let err = check_ffi(code).unwrap_err();
            let debug = format!("{err:?}");
            assert!(
                debug.starts_with(expected_variant),
                "expected {expected_variant}, got {debug}"
            );
        }
    }

    #[test]
    fn display_formats_are_human_readable() {
        let err = RustealError::PropertyNotFound("Health".into());
        assert_eq!(err.to_string(), "property not found: Health");
    }
}
