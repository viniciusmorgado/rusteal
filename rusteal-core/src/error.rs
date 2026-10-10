use std::any::Any;
use std::borrow::Cow;
use std::error::Error;
use std::fmt;
use std::panic::Location;

use std::sync::OnceLock;

use rusteal_ffi::{
    FWeakObjectHandle, RustealErrorCode, RustealErrorReport, UClassHandle, UObjectHandle,
};

use crate::traits::UeHandle;

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    ObjectDestroyed,
    InvalidCast,
    PropertyNotFound,
    FunctionNotFound,
    TypeMismatch,
    NullArgument,
    IndexOutOfRange,
    InvalidOperation,
    BufferTooSmall,
    Internal,
    Other,
}

impl ErrorKind {
    pub fn description(self) -> &'static str {
        match self {
            ErrorKind::ObjectDestroyed => "object has been destroyed",
            ErrorKind::InvalidCast => "invalid cast",
            ErrorKind::PropertyNotFound => "property not found",
            ErrorKind::FunctionNotFound => "function not found",
            ErrorKind::TypeMismatch => "type mismatch",
            ErrorKind::NullArgument => "null argument",
            ErrorKind::IndexOutOfRange => "index out of range",
            ErrorKind::InvalidOperation => "invalid operation",
            ErrorKind::BufferTooSmall => "buffer too small",
            ErrorKind::Internal => "internal error",
            ErrorKind::Other => "error",
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.description())
    }
}

impl From<RustealErrorCode> for ErrorKind {
    fn from(code: RustealErrorCode) -> Self {
        match code {
            RustealErrorCode::Ok | RustealErrorCode::InternalError => ErrorKind::Internal,
            RustealErrorCode::ObjectDestroyed => ErrorKind::ObjectDestroyed,
            RustealErrorCode::InvalidCast => ErrorKind::InvalidCast,
            RustealErrorCode::PropertyNotFound => ErrorKind::PropertyNotFound,
            RustealErrorCode::FunctionNotFound => ErrorKind::FunctionNotFound,
            RustealErrorCode::TypeMismatch => ErrorKind::TypeMismatch,
            RustealErrorCode::NullArgument => ErrorKind::NullArgument,
            RustealErrorCode::IndexOutOfRange => ErrorKind::IndexOutOfRange,
            RustealErrorCode::InvalidOperation => ErrorKind::InvalidOperation,
            RustealErrorCode::BufferTooSmall => ErrorKind::BufferTooSmall,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UeSubject {
    pub object: Option<String>,
    pub class: Option<String>,
}

impl fmt::Display for UeSubject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.object, &self.class) {
            (Some(object), Some(class)) => write!(f, "object {object} of class {class}"),
            (Some(object), None) => write!(f, "object {object}"),
            (None, Some(class)) => write!(f, "class {class}"),
            (None, None) => Ok(()),
        }
    }
}

struct Frame {
    message: Cow<'static, str>,
    location: &'static Location<'static>,
}

struct Inner {
    kind: ErrorKind,
    message: Option<Cow<'static, str>>,
    engine: Option<String>,
    subject: Option<UeSubject>,
    object: Option<FWeakObjectHandle>,
    location: &'static Location<'static>,
    frames: Vec<Frame>,
    source: Option<Box<dyn Error + Send + Sync + 'static>>,
}

pub struct RustealError {
    inner: Box<Inner>,
}

pub type RustealResult<T> = Result<T, RustealError>;

impl RustealError {
    pub(crate) fn build(kind: ErrorKind, location: &'static Location<'static>) -> Self {
        RustealError {
            inner: Box::new(Inner {
                kind,
                message: None,
                engine: None,
                subject: None,
                object: None,
                location,
                frames: Vec::new(),
                source: None,
            }),
        }
    }

    #[track_caller]
    pub fn new(kind: ErrorKind) -> Self {
        Self::build(kind, Location::caller())
    }

    #[track_caller]
    pub fn with_message(kind: ErrorKind, message: impl Into<Cow<'static, str>>) -> Self {
        let mut error = Self::build(kind, Location::caller());
        error.inner.message = Some(message.into());
        error
    }

    #[track_caller]
    pub fn msg(message: impl Into<Cow<'static, str>>) -> Self {
        Self::with_message(ErrorKind::Other, message)
    }

    #[track_caller]
    pub fn other(error: impl Error + Send + Sync + 'static) -> Self {
        into_rusteal(error, Location::caller())
    }

    #[track_caller]
    pub fn object_destroyed() -> Self {
        Self::new(ErrorKind::ObjectDestroyed)
    }

    #[track_caller]
    pub fn invalid_cast() -> Self {
        Self::new(ErrorKind::InvalidCast)
    }

    #[track_caller]
    pub fn invalid_cast_to(object: UObjectHandle, target: UClassHandle) -> Self {
        cast_failed(object, target, Location::caller())
    }

    #[track_caller]
    pub fn property_not_found(name: impl Into<Cow<'static, str>>) -> Self {
        Self::with_message(ErrorKind::PropertyNotFound, name)
    }

    #[track_caller]
    pub fn function_not_found(name: impl Into<Cow<'static, str>>) -> Self {
        Self::with_message(ErrorKind::FunctionNotFound, name)
    }

    #[track_caller]
    pub fn type_mismatch() -> Self {
        Self::new(ErrorKind::TypeMismatch)
    }

    #[track_caller]
    pub fn null_argument() -> Self {
        Self::new(ErrorKind::NullArgument)
    }

    #[track_caller]
    pub fn index_out_of_range() -> Self {
        Self::new(ErrorKind::IndexOutOfRange)
    }

    #[track_caller]
    pub fn invalid_operation(message: impl Into<Cow<'static, str>>) -> Self {
        Self::with_message(ErrorKind::InvalidOperation, message)
    }

    #[track_caller]
    pub fn internal(message: impl Into<Cow<'static, str>>) -> Self {
        Self::with_message(ErrorKind::Internal, message)
    }

    #[track_caller]
    pub fn buffer_too_small() -> Self {
        Self::new(ErrorKind::BufferTooSmall)
    }

    #[track_caller]
    pub fn context(self, context: impl Into<Cow<'static, str>>) -> Self {
        self.push_frame(context.into(), Location::caller())
    }

    fn push_frame(
        mut self,
        message: Cow<'static, str>,
        location: &'static Location<'static>,
    ) -> Self {
        self.inner.frames.push(Frame { message, location });
        self
    }

    pub fn message_or(mut self, message: impl Into<Cow<'static, str>>) -> Self {
        if self.inner.message.as_deref().is_none_or(str::is_empty) {
            self.inner.message = Some(message.into());
        }

        self
    }

    pub fn with_source(mut self, source: impl Error + Send + Sync + 'static) -> Self {
        self.inner.source = Some(Box::new(source));
        self
    }

    pub fn on(self, object: &impl UeHandle) -> Self {
        self.on_handle(object.raw_handle())
    }

    pub fn on_handle(mut self, object: UObjectHandle) -> Self {
        if self.inner.subject.is_none() {
            let subject = UeSubject {
                object: object_name(object),
                class: class_name(class_of(object)),
            };

            if subject.object.is_some() {
                self.inner.object = Some(unsafe { crate::ffi_dispatch::core_make_weak(object) });
            }

            if subject.object.is_some() || subject.class.is_some() {
                self.inner.subject = Some(subject);
            }
        }

        self
    }

    pub fn on_class(mut self, class: UClassHandle) -> Self {
        if self.inner.subject.is_none() {
            self.inner.subject = class_name(class).map(|class| UeSubject {
                object: None,
                class: Some(class),
            });
        }

        self
    }

    pub fn kind(&self) -> ErrorKind {
        self.inner.kind
    }

    pub fn is(&self, kind: ErrorKind) -> bool {
        self.inner.kind == kind
    }

    pub fn message(&self) -> Option<&str> {
        self.inner.message.as_deref().filter(|m| !m.is_empty())
    }

    pub fn subject(&self) -> Option<&UeSubject> {
        self.inner.subject.as_ref()
    }

    pub fn location(&self) -> &'static Location<'static> {
        self.inner.location
    }

    pub fn contexts(&self) -> impl Iterator<Item = (&str, &'static Location<'static>)> {
        self.inner
            .frames
            .iter()
            .rev()
            .map(|frame| (frame.message.as_ref(), frame.location))
    }

    pub fn causes(&self) -> impl Iterator<Item = &(dyn Error + 'static)> {
        let mut next = self.source();

        std::iter::from_fn(move || {
            let current = next?;
            next = current.source();
            Some(current)
        })
    }

    pub fn report(&self) -> String {
        format!("{self:?}")
    }

    pub fn engine_message(&self) -> Option<&str> {
        self.inner.engine.as_deref()
    }

    pub fn source_file(&self) -> String {
        absolute_path(self.inner.location.file())
    }

    pub fn log(&self, level: u8) {
        self.send(level, false, None);
    }

    #[doc(hidden)]
    pub fn __raise(&self, function: &str) {
        self.send(crate::logging::LOG_ERROR, true, Some(function));
    }

    #[doc(hidden)]
    pub fn __log_in(&self, level: u8, what: &str) {
        self.send(level, false, Some(what));
    }

    fn send(&self, level: u8, script_error: bool, function: Option<&str>) {
        if !crate::api::is_api_initialized() {
            return;
        }

        let summary = match function {
            Some(function) => format!("{function} failed: {self}"),
            None => self.to_string(),
        };

        let details = match function {
            Some(function) => format!("{function} failed: {self:?}"),
            None => format!("{self:?}"),
        };

        let file = self.source_file();

        let report = RustealErrorReport {
            level,
            script_error,
            summary: summary.as_ptr(),
            summary_len: summary.len() as u32,
            details: details.as_ptr(),
            details_len: details.len() as u32,
            object: self.inner.object.unwrap_or(FWeakObjectHandle {
                object_index: -1,
                object_serial_number: 0,
            }),
            file: file.as_ptr(),
            file_len: file.len() as u32,
            line: self.inner.location.line(),
            column: self.inner.location.column(),
        };

        unsafe {
            crate::ffi_dispatch::logging_report(&report);
        }
    }

    fn wraps_its_source(&self) -> bool {
        self.inner.kind == ErrorKind::Other
            && self.message().is_none()
            && self.inner.source.is_some()
    }

    fn shown_causes(&self) -> impl Iterator<Item = &(dyn Error + 'static)> {
        self.causes().skip(usize::from(self.wraps_its_source()))
    }

    fn fmt_base(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (
            self.inner.kind,
            self.message(),
            self.inner.source.as_deref(),
        ) {
            (ErrorKind::Other, Some(message), _) => f.write_str(message)?,
            (ErrorKind::Other, None, Some(source)) => write!(f, "{source}")?,
            (kind, Some(message), _) => write!(f, "{kind}: {message}")?,
            (kind, None, _) => write!(f, "{kind}")?,
        }

        if let Some(subject) = &self.inner.subject {
            write!(f, " ({subject})")?;
        }

        if let Some(engine) = &self.inner.engine {
            write!(f, ": Unreal: {engine}")?;
        }

        Ok(())
    }
}

impl PartialEq<ErrorKind> for RustealError {
    fn eq(&self, kind: &ErrorKind) -> bool {
        self.inner.kind == *kind
    }
}

impl fmt::Display for RustealError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (context, _) in self.contexts() {
            write!(f, "{context}: ")?;
        }

        self.fmt_base(f)?;

        if f.alternate() {
            for cause in self.shown_causes() {
                write!(f, ": {cause}")?;
            }

            write!(f, " [{}]", short_location(self.location()))?;
        }

        Ok(())
    }
}

impl fmt::Debug for RustealError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_base(f)?;
        write!(f, "\n    at {}", short_location(self.location()))?;

        let mut last = self.location();

        for frame in &self.inner.frames {
            write!(f, "\nwhile {}", frame.message)?;

            if frame.location != last {
                write!(f, "\n    at {}", short_location(frame.location))?;
            }

            last = frame.location;
        }

        for cause in self.shown_causes() {
            write!(f, "\ncaused by: {cause}")?;
        }

        Ok(())
    }
}

impl Error for RustealError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner
            .source
            .as_deref()
            .map(|source| source as &(dyn Error + 'static))
    }
}

impl From<ErrorKind> for RustealError {
    #[track_caller]
    fn from(kind: ErrorKind) -> Self {
        Self::build(kind, Location::caller())
    }
}

impl From<RustealErrorCode> for RustealError {
    #[track_caller]
    fn from(code: RustealErrorCode) -> Self {
        let error = from_code(code, Location::caller());

        match code {
            RustealErrorCode::Ok => error.message_or("unexpected Ok error code"),
            _ => error,
        }
    }
}

macro_rules! from_std_errors {
    ($($error:ty),* $(,)?) => {
        $(
            impl From<$error> for RustealError {
                #[track_caller]
                fn from(error: $error) -> Self {
                    into_rusteal(error, Location::caller())
                }
            }
        )*
    };
}

from_std_errors!(
    std::io::Error,
    std::fmt::Error,
    std::str::Utf8Error,
    std::string::FromUtf8Error,
    std::num::ParseIntError,
    std::num::ParseFloatError,
    std::num::TryFromIntError,
    std::str::ParseBoolError,
    std::char::ParseCharError,
);

impl From<Box<dyn Error + Send + Sync + 'static>> for RustealError {
    #[track_caller]
    fn from(error: Box<dyn Error + Send + Sync + 'static>) -> Self {
        match error.downcast::<RustealError>() {
            Ok(error) => *error,
            Err(error) => {
                let mut wrapped = Self::build(ErrorKind::Other, Location::caller());
                wrapped.inner.source = Some(error);
                wrapped
            }
        }
    }
}

fn into_rusteal<E: Error + Send + Sync + 'static>(
    error: E,
    location: &'static Location<'static>,
) -> RustealError {
    let mut slot = Some(error);

    if let Some(own) = (&mut slot as &mut dyn Any).downcast_mut::<Option<RustealError>>() {
        return own.take().expect("the slot holds the error");
    }

    let error = slot.take().expect("the slot holds the error");
    let mut wrapped = RustealError::build(ErrorKind::Other, location);
    wrapped.inner.source = Some(Box::new(error));
    wrapped
}

pub trait ErrorContext<T> {
    fn context(self, context: impl Into<Cow<'static, str>>) -> RustealResult<T>;

    fn with_context<C, F>(self, context: F) -> RustealResult<T>
    where
        C: Into<Cow<'static, str>>,
        F: FnOnce() -> C;
}

impl<T, E: Error + Send + Sync + 'static> ErrorContext<T> for Result<T, E> {
    #[track_caller]
    fn context(self, context: impl Into<Cow<'static, str>>) -> RustealResult<T> {
        let location = Location::caller();

        self.map_err(|error| into_rusteal(error, location).push_frame(context.into(), location))
    }

    #[track_caller]
    fn with_context<C, F>(self, context: F) -> RustealResult<T>
    where
        C: Into<Cow<'static, str>>,
        F: FnOnce() -> C,
    {
        let location = Location::caller();

        self.map_err(|error| into_rusteal(error, location).push_frame(context().into(), location))
    }
}

impl<T> ErrorContext<T> for Option<T> {
    #[track_caller]
    fn context(self, context: impl Into<Cow<'static, str>>) -> RustealResult<T> {
        let location = Location::caller();

        self.ok_or_else(|| RustealError::build(ErrorKind::Other, location).message_or(context))
    }

    #[track_caller]
    fn with_context<C, F>(self, context: F) -> RustealResult<T>
    where
        C: Into<Cow<'static, str>>,
        F: FnOnce() -> C,
    {
        let location = Location::caller();

        self.ok_or_else(|| RustealError::build(ErrorKind::Other, location).message_or(context()))
    }
}

#[macro_export]
macro_rules! bail {
    ($kind:path, $($arg:tt)+) => {
        return ::core::result::Result::Err($crate::RustealError::with_message($kind, format!($($arg)+)).into())
    };
    ($($arg:tt)+) => {
        return ::core::result::Result::Err($crate::RustealError::msg(format!($($arg)+)).into())
    };
}

#[macro_export]
macro_rules! ensure {
    ($condition:expr, $($arg:tt)+) => {
        if !$condition {
            $crate::bail!($($arg)+);
        }
    };
}

fn short_location(location: &Location<'_>) -> String {
    let file = location.file();

    let short = ["/src/", "\\src\\"]
        .iter()
        .filter_map(|marker| file.rfind(marker))
        .max()
        .and_then(|src| {
            let crate_start = file[..src].rfind(['/', '\\']).map_or(0, |i| i + 1);
            file.get(crate_start..)
        })
        .unwrap_or(file);

    format!("{short}:{}:{}", location.line(), location.column())
}

static SOURCE_ROOT: OnceLock<String> = OnceLock::new();

#[doc(hidden)]
pub fn set_source_root(crate_dir: &str) {
    let root = std::path::Path::new(crate_dir)
        .parent()
        .unwrap_or(std::path::Path::new(crate_dir));

    let _ = SOURCE_ROOT.set(root.to_string_lossy().into_owned());
}

pub(crate) fn absolute_path(file: &str) -> String {
    let path = std::path::Path::new(file);

    match SOURCE_ROOT.get() {
        Some(root) if path.is_relative() => std::path::Path::new(root)
            .join(path)
            .to_string_lossy()
            .into_owned(),
        _ => file.to_owned(),
    }
}

#[cold]
fn from_code(code: RustealErrorCode, location: &'static Location<'static>) -> RustealError {
    let mut error = RustealError::build(code.into(), location);

    if code != RustealErrorCode::ObjectDestroyed {
        error.inner.engine = engine_message(code);
    }

    error
}

fn engine_message(code: RustealErrorCode) -> Option<String> {
    if !crate::api::is_api_initialized() {
        return None;
    }

    let mut buf = vec![0u8; 512];
    let mut len: u32 = 0;

    let found = unsafe {
        crate::ffi_dispatch::core_take_error_message(
            code,
            buf.as_mut_ptr(),
            buf.len() as u32,
            &mut len,
        )
    };

    if !found {
        return None;
    }

    if len as usize > buf.len() {
        buf.resize(len as usize, 0);

        unsafe {
            crate::ffi_dispatch::core_take_error_message(code, buf.as_mut_ptr(), len, &mut len);
        }
    }

    buf.truncate(len as usize);
    Some(String::from_utf8_lossy(&buf).into_owned())
}

#[cold]
pub(crate) fn destroyed<T: ?Sized>(location: &'static Location<'static>) -> RustealError {
    RustealError::build(ErrorKind::ObjectDestroyed, location)
        .message_or(format!("UObjectRef<{}>", short_type_name::<T>()))
}

#[cold]
pub(crate) fn cast_failed(
    object: UObjectHandle,
    target: UClassHandle,
    location: &'static Location<'static>,
) -> RustealError {
    let error = RustealError::build(ErrorKind::InvalidCast, location).on_handle(object);

    match class_name(target) {
        Some(target) => error.message_or(format!("expected {target}")),
        None => error,
    }
}

pub(crate) fn short_type_name<T: ?Sized>() -> &'static str {
    let full = std::any::type_name::<T>();
    let base = full.split('<').next().unwrap_or(full);
    let start = base.rfind("::").map_or(0, |i| i + 2);
    &full[start..]
}

pub(crate) fn object_name(object: UObjectHandle) -> Option<String> {
    if object.is_null()
        || !crate::api::is_api_initialized()
        || !unsafe { crate::ffi_dispatch::core_is_valid(object) }
    {
        return None;
    }

    let mut len: u32 = 0;

    let code =
        unsafe { crate::ffi_dispatch::core_get_name(object, std::ptr::null_mut(), 0, &mut len) };

    if code != RustealErrorCode::Ok {
        return None;
    }

    let mut buf = vec![0u8; len as usize];

    let code =
        unsafe { crate::ffi_dispatch::core_get_name(object, buf.as_mut_ptr(), len, &mut len) };

    (code == RustealErrorCode::Ok).then(|| {
        buf.truncate(len as usize);
        String::from_utf8_lossy(&buf).into_owned()
    })
}

fn class_of(object: UObjectHandle) -> UClassHandle {
    if object.is_null()
        || !crate::api::is_api_initialized()
        || !unsafe { crate::ffi_dispatch::core_is_valid(object) }
    {
        return UClassHandle::null();
    }

    unsafe { crate::ffi_dispatch::core_get_class(object) }
}

pub(crate) fn class_name(class: UClassHandle) -> Option<String> {
    object_name(UObjectHandle(class.0))
}

#[track_caller]
pub fn check_ffi(code: RustealErrorCode) -> RustealResult<()> {
    match code {
        RustealErrorCode::Ok => Ok(()),
        other => Err(from_code(other, Location::caller())),
    }
}

#[track_caller]
pub fn check_ffi_ctx(code: RustealErrorCode, context: &str) -> RustealResult<()> {
    match code {
        RustealErrorCode::Ok => Ok(()),
        other => {
            let error = from_code(other, Location::caller());

            Err(match other {
                RustealErrorCode::PropertyNotFound
                | RustealErrorCode::FunctionNotFound
                | RustealErrorCode::InvalidOperation => error.message_or(context.to_owned()),
                _ => error.context(context.to_owned()),
            })
        }
    }
}

#[inline(always)]
pub fn ffi_infallible(code: RustealErrorCode) {
    debug_assert_eq!(
        code,
        RustealErrorCode::Ok,
        "FFI call returned {code:?} after pre-validation"
    );
}

#[inline(always)]
pub fn ffi_infallible_ctx(code: RustealErrorCode, ctx: &str) {
    debug_assert_eq!(
        code,
        RustealErrorCode::Ok,
        "FFI '{ctx}' returned {code:?} after pre-validation"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_ffi_ok_returns_ok() {
        assert!(check_ffi(RustealErrorCode::Ok).is_ok());
    }

    #[test]
    fn check_ffi_errors_map_to_their_kind() {
        let cases = [
            (
                RustealErrorCode::ObjectDestroyed,
                ErrorKind::ObjectDestroyed,
            ),
            (RustealErrorCode::InvalidCast, ErrorKind::InvalidCast),
            (
                RustealErrorCode::PropertyNotFound,
                ErrorKind::PropertyNotFound,
            ),
            (
                RustealErrorCode::FunctionNotFound,
                ErrorKind::FunctionNotFound,
            ),
            (RustealErrorCode::TypeMismatch, ErrorKind::TypeMismatch),
            (RustealErrorCode::NullArgument, ErrorKind::NullArgument),
            (
                RustealErrorCode::IndexOutOfRange,
                ErrorKind::IndexOutOfRange,
            ),
            (
                RustealErrorCode::InvalidOperation,
                ErrorKind::InvalidOperation,
            ),
            (RustealErrorCode::InternalError, ErrorKind::Internal),
            (RustealErrorCode::BufferTooSmall, ErrorKind::BufferTooSmall),
        ];

        for (code, kind) in cases {
            let error = check_ffi(code).unwrap_err();
            assert_eq!(error.kind(), kind);
            assert!(error == kind);
        }
    }

    #[test]
    fn the_name_given_to_check_ffi_ctx_is_the_message() {
        let error = check_ffi_ctx(RustealErrorCode::PropertyNotFound, "Health").unwrap_err();
        assert_eq!(error.to_string(), "property not found: Health");

        let error = check_ffi_ctx(RustealErrorCode::TypeMismatch, "Health").unwrap_err();
        assert_eq!(error.to_string(), "Health: type mismatch");
    }

    #[test]
    fn display_is_one_human_readable_line() {
        assert_eq!(
            RustealError::property_not_found("Health").to_string(),
            "property not found: Health"
        );

        assert_eq!(RustealError::invalid_cast().to_string(), "invalid cast");
        assert_eq!(
            RustealError::msg("no save slot").to_string(),
            "no save slot"
        );
    }

    #[test]
    fn the_location_is_where_the_error_was_made() {
        let line = line!() + 1;
        let error = RustealError::null_argument();
        assert_eq!(error.location().line(), line);
        assert!(error.location().file().ends_with("error.rs"));
    }

    fn parse(text: &str) -> (u32, RustealResult<i32>) {
        let line = line!() + 1;
        let result = (|| -> RustealResult<i32> { Ok(text.parse::<i32>()?) })();
        (line, result)
    }

    #[test]
    fn question_mark_converts_std_errors_where_it_is() {
        let (line, result) = parse("x");
        let error = result.unwrap_err();

        assert_eq!(error.kind(), ErrorKind::Other);
        assert_eq!(error.location().line(), line);
        assert_eq!(error.causes().count(), 1);
        assert_eq!(error.to_string(), "invalid digit found in string");
    }

    #[test]
    fn context_frames_read_outermost_first() {
        let error = Err::<(), _>(RustealError::property_not_found("Health"))
            .context("reading the enemy")
            .context("applying damage")
            .unwrap_err();

        assert_eq!(error.kind(), ErrorKind::PropertyNotFound);

        assert_eq!(
            error.to_string(),
            "applying damage: reading the enemy: property not found: Health"
        );

        let contexts: Vec<&str> = error.contexts().map(|(c, _)| c).collect();
        assert_eq!(contexts, ["applying damage", "reading the enemy"]);
    }

    #[test]
    fn context_wraps_foreign_errors_as_their_cause() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "SAVE.sav missing");

        let error = Err::<(), _>(io).context("loading the save").unwrap_err();

        assert_eq!(error.kind(), ErrorKind::Other);
        assert_eq!(error.to_string(), "loading the save: SAVE.sav missing");
        assert_eq!(
            error.causes().next().unwrap().to_string(),
            "SAVE.sav missing"
        );

        let alternate = format!("{error:#}");
        assert!(alternate.starts_with("loading the save: SAVE.sav missing ["));
        assert!(alternate.contains("rusteal-core/src/error.rs:"));
    }

    #[test]
    fn option_context_makes_an_error() {
        let error = None::<i32>.context("no player pawn").unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Other);
        assert_eq!(error.to_string(), "no player pawn");
    }

    #[test]
    fn debug_is_a_report_with_locations() {
        let error = Err::<(), _>(RustealError::function_not_found("Interact"))
            .context("using the door")
            .unwrap_err();

        let report = format!("{error:?}");
        let lines: Vec<&str> = report.lines().collect();

        assert_eq!(lines[0], "function not found: Interact");
        assert!(lines[1].starts_with("    at rusteal-core/src/error.rs:"));
        assert_eq!(lines[2], "while using the door");
        assert!(lines[3].starts_with("    at rusteal-core/src/error.rs:"));
    }

    #[test]
    fn a_context_on_the_line_of_the_error_gives_no_second_location() {
        let error = "x".parse::<i32>().context("reading the ammo").unwrap_err();
        let report = format!("{error:?}");

        assert_eq!(report.matches("    at ").count(), 1, "{report}");
        assert!(report.contains("\nwhile reading the ammo"));
    }

    #[test]
    fn a_rusteal_error_given_to_other_or_boxed_stays_itself() {
        let original = RustealError::index_out_of_range().context("slot 9");
        let line = original.location().line();
        let error = RustealError::other(original);
        assert_eq!(error.kind(), ErrorKind::IndexOutOfRange);
        assert_eq!(error.location().line(), line);

        let boxed: Box<dyn Error + Send + Sync> = Box::new(RustealError::type_mismatch());
        assert_eq!(RustealError::from(boxed).kind(), ErrorKind::TypeMismatch);
    }

    #[test]
    fn subjects_name_the_object_and_its_class() {
        let subject = UeSubject {
            object: Some("BP_Enemy_C_0".into()),
            class: Some("BP_Enemy_C".into()),
        };

        let mut error = RustealError::property_not_found("Health");
        error.inner.subject = Some(subject);

        assert_eq!(
            error.to_string(),
            "property not found: Health (object BP_Enemy_C_0 of class BP_Enemy_C)"
        );
    }

    #[test]
    fn the_engine_reason_follows_the_object() {
        let mut error = RustealError::type_mismatch();
        error.inner.subject = Some(UeSubject {
            object: Some("BP_Enemy_C_0".into()),
            class: Some("BP_Enemy_C".into()),
        });
        error.inner.engine =
            Some("Health, a FloatProperty of BP_Enemy_C, is not a FBoolProperty".into());

        assert_eq!(
            error.to_string(),
            "type mismatch (object BP_Enemy_C_0 of class BP_Enemy_C): Unreal: Health, a \
             FloatProperty of BP_Enemy_C, is not a FBoolProperty"
        );
        assert_eq!(error.engine_message().unwrap().len(), 61);
    }

    #[test]
    fn source_files_are_made_absolute_from_the_crate_root() {
        set_source_root("/project/Rust/my-game");
        assert_eq!(
            absolute_path("my-game/src/turret.rs"),
            "/project/Rust/my-game/src/turret.rs"
        );
        assert_eq!(
            absolute_path("/abs/rusteal-core/src/x.rs"),
            "/abs/rusteal-core/src/x.rs"
        );
    }

    #[test]
    fn without_the_engine_no_subject_is_added() {
        let error = RustealError::invalid_cast().on_handle(UObjectHandle::null());
        assert!(error.subject().is_none());
    }

    #[test]
    fn bail_and_ensure_return_errors() {
        fn check(health: i32) -> RustealResult<i32> {
            ensure!(health >= 0, "health is {health}");

            if health > 100 {
                bail!(
                    ErrorKind::InvalidOperation,
                    "health {health} over the maximum"
                );
            }

            Ok(health)
        }

        assert_eq!(check(5).unwrap(), 5);
        assert_eq!(check(-1).unwrap_err().to_string(), "health is -1");

        assert_eq!(
            check(101).unwrap_err().to_string(),
            "invalid operation: health 101 over the maximum"
        );
    }

    #[test]
    fn locations_are_shortened_to_the_crate() {
        let location = Location::caller();
        assert!(short_location(location).starts_with("rusteal-core/src/error.rs:"));
    }

    #[test]
    fn the_error_is_one_pointer_and_thread_safe() {
        fn assert_send_sync<T: Send + Sync + 'static>() {}
        assert_send_sync::<RustealError>();
        assert_eq!(size_of::<RustealResult<()>>(), size_of::<usize>());
    }
}
