use std::fmt;
use std::ptr::NonNull;

use rusteal_ffi::UObjectHandle;

use crate::containers::ContainerElement;
use crate::ffi_dispatch;
use crate::object_ref::ObjectPointer;
use crate::strings::take_utf8;

pub struct Text {
    raw: NonNull<u8>,
}

unsafe impl Send for Text {}

impl Text {
    pub fn from_string(value: &str) -> Self {
        unsafe {
            Self::from_raw(ffi_dispatch::core_text_from_string(
                value.as_ptr(),
                value.len() as u32,
            ))
        }
    }

    pub fn localized(namespace: &str, key: &str, source: &str) -> Self {
        unsafe {
            Self::from_raw(ffi_dispatch::core_text_localized(
                namespace.as_ptr(),
                namespace.len() as u32,
                key.as_ptr(),
                key.len() as u32,
                source.as_ptr(),
                source.len() as u32,
            ))
        }
    }

    pub fn empty() -> Self {
        Self::from_string("")
    }

    pub fn is_empty(&self) -> bool {
        unsafe { ffi_dispatch::core_text_is_empty(self.as_raw()) }
    }

    /// # Safety
    ///
    /// `raw` is an `FText` the engine allocated for Rust, which this value now owns.
    #[doc(hidden)]
    pub unsafe fn from_raw(raw: *mut u8) -> Self {
        Text {
            raw: NonNull::new(raw).expect("the engine gave a null FText"),
        }
    }

    #[doc(hidden)]
    pub fn as_raw(&self) -> *const u8 {
        self.raw.as_ptr()
    }

    #[doc(hidden)]
    pub fn as_mut_raw(&mut self) -> *mut u8 {
        self.raw.as_ptr()
    }
}

impl Clone for Text {
    fn clone(&self) -> Self {
        unsafe { Self::from_raw(ffi_dispatch::core_text_clone(self.as_raw())) }
    }
}

impl Drop for Text {
    fn drop(&mut self) {
        unsafe { ffi_dispatch::core_text_destroy(self.raw.as_ptr()) }
    }
}

impl Default for Text {
    fn default() -> Self {
        Self::empty()
    }
}

impl PartialEq for Text {
    fn eq(&self, other: &Self) -> bool {
        unsafe { ffi_dispatch::core_text_equal(self.as_raw(), other.as_raw()) }
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = vec![0u8; 256];
        let mut len: u32 = 0;

        unsafe {
            ffi_dispatch::core_text_to_string(
                self.as_raw(),
                buf.as_mut_ptr(),
                buf.len() as u32,
                &mut len,
            );
        }

        f.write_str(&take_utf8(buf, len))
    }
}

impl fmt::Debug for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Text").field(&self.to_string()).finish()
    }
}

impl From<&str> for Text {
    fn from(value: &str) -> Self {
        Self::from_string(value)
    }
}

impl From<&String> for Text {
    fn from(value: &String) -> Self {
        Self::from_string(value)
    }
}

impl From<String> for Text {
    fn from(value: String) -> Self {
        Self::from_string(&value)
    }
}

impl From<&Text> for Text {
    fn from(value: &Text) -> Self {
        value.clone()
    }
}

impl ObjectPointer for Text {
    unsafe fn from_object_handle(handle: UObjectHandle) -> Self {
        unsafe { Self::from_raw(handle.0 as *mut u8) }
    }

    fn object_handle(&self) -> UObjectHandle {
        UObjectHandle(self.raw.as_ptr() as *mut std::ffi::c_void)
    }
}

unsafe impl ContainerElement for Text {
    const BUF_SIZE: u32 = std::mem::size_of::<*mut u8>() as u32;

    unsafe fn read_from_buf(buf: *const u8, _written: u32) -> Self {
        unsafe { Self::from_raw((buf as *const *mut u8).read_unaligned()) }
    }

    unsafe fn write_to_buf(&self, buf: *mut u8) -> u32 {
        unsafe {
            (buf as *mut *const u8).write_unaligned(self.as_raw());
        }

        std::mem::size_of::<*mut u8>() as u32
    }
}
