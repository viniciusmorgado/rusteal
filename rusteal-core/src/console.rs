use rusteal_ffi::{RustealConsoleArgs, UObjectHandle};

use crate::error::{RustealError, RustealResult, check_ffi_ctx};
use crate::{delegate_registry, ffi_dispatch};

pub fn register_command(
    name: &str,
    help: &str,
    mut run: impl FnMut(&[&str], UObjectHandle) + Send + 'static,
) -> RustealResult<()> {
    let callback_id = delegate_registry::register_callback(move |params| {
        let args = unsafe { &*(params as *const RustealConsoleArgs) };

        let text = if args.args.is_null() {
            ""
        } else {
            let bytes = unsafe { std::slice::from_raw_parts(args.args, args.args_len as usize) };

            std::str::from_utf8(bytes).unwrap_or("")
        };

        let words: Vec<&str> = if text.is_empty() {
            Vec::new()
        } else {
            text.split('\n').collect()
        };

        run(&words, args.world);
    });

    let code = unsafe {
        ffi_dispatch::console_register_command(
            name.as_ptr(),
            name.len() as u32,
            help.as_ptr(),
            help.len() as u32,
            callback_id,
        )
    };

    let result = check_ffi_ctx(code, name);

    if result.is_err() {
        delegate_registry::unregister_callback(callback_id);
    }

    result
}

pub trait ConsoleValue: Sized {
    #[doc(hidden)]
    const KIND: u32;
    fn to_text(&self) -> String;
    fn from_text(text: &str) -> Option<Self>;
}

impl ConsoleValue for bool {
    const KIND: u32 = 0;
    fn to_text(&self) -> String {
        if *self { "1" } else { "0" }.to_string()
    }
    fn from_text(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" | "" => Some(false),
            other => other.parse::<f64>().ok().map(|n| n != 0.0),
        }
    }
}

impl ConsoleValue for i32 {
    const KIND: u32 = 1;
    fn to_text(&self) -> String {
        self.to_string()
    }
    fn from_text(text: &str) -> Option<Self> {
        let text = text.trim();

        text.parse()
            .ok()
            .or_else(|| text.parse::<f64>().ok().map(|n| n as i32))
    }
}

impl ConsoleValue for f32 {
    const KIND: u32 = 2;
    fn to_text(&self) -> String {
        self.to_string()
    }
    fn from_text(text: &str) -> Option<Self> {
        text.trim().parse().ok()
    }
}

impl ConsoleValue for String {
    const KIND: u32 = 3;
    fn to_text(&self) -> String {
        self.clone()
    }
    fn from_text(text: &str) -> Option<Self> {
        Some(text.to_string())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConsoleVariable<T> {
    name: &'static str,
    _value: std::marker::PhantomData<fn() -> T>,
}

impl<T: ConsoleValue> ConsoleVariable<T> {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            _value: std::marker::PhantomData,
        }
    }

    pub fn register(&self, help: &str, default: T) -> RustealResult<()> {
        let default = default.to_text();

        let code = unsafe {
            ffi_dispatch::console_register_variable(
                self.name.as_ptr(),
                self.name.len() as u32,
                help.as_ptr(),
                help.len() as u32,
                T::KIND,
                default.as_ptr(),
                default.len() as u32,
            )
        };

        check_ffi_ctx(code, self.name)
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn get(&self) -> RustealResult<T> {
        let text = get_variable_text(self.name)?;

        T::from_text(&text).ok_or_else(|| {
            RustealError::Internal(format!("console variable {} holds '{text}'", self.name))
        })
    }

    pub fn set(&self, value: T) -> RustealResult<()> {
        set_variable_text(self.name, &value.to_text())
    }
}

pub fn unregister(name: &str) -> RustealResult<()> {
    check_ffi_ctx(
        unsafe { ffi_dispatch::console_unregister(name.as_ptr(), name.len() as u32) },
        name,
    )
}

pub fn get_variable_text(name: &str) -> RustealResult<String> {
    let mut buf = vec![0u8; 256];

    loop {
        let mut len: u32 = 0;

        let code = unsafe {
            ffi_dispatch::console_get_variable(
                name.as_ptr(),
                name.len() as u32,
                buf.as_mut_ptr(),
                buf.len() as u32,
                &mut len,
            )
        };

        check_ffi_ctx(code, name)?;

        if (len as usize) <= buf.len() {
            buf.truncate(len as usize);

            return String::from_utf8(buf)
                .map_err(|_| RustealError::Internal("console value is not UTF-8".into()));
        }

        buf.resize(len as usize, 0);
    }
}

pub fn set_variable_text(name: &str, value: &str) -> RustealResult<()> {
    check_ffi_ctx(
        unsafe {
            ffi_dispatch::console_set_variable(
                name.as_ptr(),
                name.len() as u32,
                value.as_ptr(),
                value.len() as u32,
            )
        },
        name,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_read_the_console_text() {
        assert_eq!(bool::from_text("1"), Some(true));
        assert_eq!(bool::from_text("False"), Some(false));
        assert_eq!(bool::from_text("2"), Some(true));
        assert_eq!(i32::from_text("42"), Some(42));
        assert_eq!(i32::from_text("3.0"), Some(3));
        assert_eq!(f32::from_text("0.5"), Some(0.5));
        assert_eq!(f32::from_text("x"), None);
        assert_eq!(String::from_text("a b"), Some("a b".to_string()));
        assert_eq!(true.to_text(), "1");
        assert_eq!(7.to_text(), "7");
    }
}
