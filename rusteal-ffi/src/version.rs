// The Rusteal version as an integer: major * 1_000_000 + minor * 1_000 + patch
// (0.2.1 -> 2001). Crates, the UE plugins and this contract always carry the
// same version. The plugin checks the library's `rusteal_version()` export
// before calling `rusteal_init`, and writes its own version into
// `RustealApiTable::version`, which the library checks again.

/// The version of this crate, encoded.
pub const RUSTEAL_VERSION: u32 = encode_version(env!("CARGO_PKG_VERSION"));

/// Encode `major.minor.patch`. Anything else is a panic — at compile time for
/// [`RUSTEAL_VERSION`].
pub const fn encode_version(version: &str) -> u32 {
    let bytes = version.as_bytes();
    let mut parts = [0u32; 3];
    let mut part = 0;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'.' {
            part += 1;
            assert!(part < 3, "version must be major.minor.patch");
        } else {
            assert!(b.is_ascii_digit(), "version must be major.minor.patch");
            parts[part] = parts[part] * 10 + (b - b'0') as u32;
        }
        i += 1;
    }
    assert!(part == 2, "version must be major.minor.patch");
    assert!(parts[1] < 1_000 && parts[2] < 1_000, "minor and patch must be below 1000");
    parts[0] * 1_000_000 + parts[1] * 1_000 + parts[2]
}

/// `major.minor.patch` back from an encoded version, for messages.
pub fn decode_version(version: u32) -> String {
    format!(
        "{}.{}.{}",
        version / 1_000_000,
        version / 1_000 % 1_000,
        version % 1_000
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_major_minor_patch() {
        assert_eq!(encode_version("0.2.1"), 2001);
        assert_eq!(encode_version("0.3.0"), 3000);
        assert_eq!(encode_version("1.0.0"), 1_000_000);
        assert_eq!(encode_version("12.345.678"), 12_345_678);
    }

    #[test]
    fn decodes_back() {
        for v in ["0.2.1", "0.3.0", "1.0.0", "12.345.678"] {
            assert_eq!(decode_version(encode_version(v)), v);
        }
    }

    #[test]
    fn this_crate_is_encoded() {
        assert_eq!(decode_version(RUSTEAL_VERSION), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    #[should_panic]
    fn rejects_two_parts() {
        encode_version("0.2");
    }

    #[test]
    #[should_panic]
    fn rejects_prerelease() {
        encode_version("0.2.1-alpha");
    }
}
