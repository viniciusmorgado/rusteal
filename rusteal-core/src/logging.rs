pub const LOG_DISPLAY: u8 = 0;
pub const LOG_WARNING: u8 = 1;
pub const LOG_ERROR: u8 = 2;

#[macro_export]
macro_rules! ulog {
    ($level:expr, $($arg:tt)*) => {{
        let msg = format!($($arg)*);
        let bytes = msg.as_bytes();

        unsafe {
            $crate::ffi_dispatch::logging_log($level, bytes.as_ptr(), bytes.len() as u32);
        }
    }};
}
