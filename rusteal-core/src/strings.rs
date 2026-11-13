use crate::ffi_dispatch;

pub fn take_utf8(buf: Vec<u8>, len: u32) -> String {
    let len = len as usize;

    let bytes = if len > buf.len() {
        let mut full = vec![0u8; len];

        unsafe {
            ffi_dispatch::core_take_pending_utf8(buf.as_ptr(), full.as_mut_ptr(), len as u32);
        }

        full
    } else {
        let mut buf = buf;
        buf.truncate(len);
        buf
    };

    String::from_utf8(bytes).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}
