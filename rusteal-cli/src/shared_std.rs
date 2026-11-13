//! `rusteal package --static --feature shared-std`: a static build for an engine that links Rust of its own.
//!
//! The static library keeps Rust's standard library inside it: fat LTO compiles std into the library's one object and
//! hides it, so the game can link it next to any other Rust code. One symbol stays visible, the unwinding personality
//! (`rust_eh_personality`). An engine that links its own Rust static library defines it too, and the final link of the
//! game fails with a duplicate symbol. This feature takes the library's personality out of the way, in the form each
//! platform's object files allow:
//!
//! - ELF (Linux, Android) and Mach-O (macOS, iOS): the symbol becomes a weak definition, so the linker takes the
//!   engine's and both halves of the game unwind through it. The personality only reads the unwinding tables the
//!   compiler wrote into each function, so it serves both, and each half still catches its own panics.
//! - COFF (Windows): COFF has no weak definitions, so the symbol becomes local to its object. Rust on Windows unwinds
//!   through the C++ runtime's handler and never calls this symbol; each half keeps its own.
//!
//! The change is made on the installed archive, in place: one field of the symbol's entry, so no offset in the file
//! moves. A universal (fat) Mach-O library is changed in each of its architectures.

use std::fs;
use std::path::Path;

pub const SHARED_SYMBOL: &str = "rust_eh_personality";

const ARCHIVE_MAGIC: &[u8] = b"!<arch>\n";

pub fn share_engine_std(library: &Path) -> Result<usize, String> {
    let mut bytes =
        fs::read(library).map_err(|e| format!("cannot read {}: {e}", library.display()))?;

    let shared = share_in_library(&mut bytes)
        .map_err(|e| format!("shared-std: {}: {e}", library.display()))?;

    if shared > 0 {
        fs::write(library, &bytes)
            .map_err(|e| format!("cannot write {}: {e}", library.display()))?;
    }

    Ok(shared)
}

/// Gives way to the engine's personality in a static library (`.a` or `.lib`), or in each architecture of a universal
/// Mach-O library. Returns how many symbols changed; a library already changed gives 0.
pub fn share_in_library(bytes: &mut [u8]) -> Result<usize, String> {
    if bytes.starts_with(ARCHIVE_MAGIC) {
        return share_in_archive(bytes);
    }

    match bytes.get(..4) {
        Some([0xca, 0xfe, 0xba, 0xbe]) => share_in_universal(bytes, false),
        Some([0xca, 0xfe, 0xba, 0xbf]) => share_in_universal(bytes, true),
        _ => Err("not a static library".into()),
    }
}

fn share_in_universal(bytes: &mut [u8], wide: bool) -> Result<usize, String> {
    let be32 = |bytes: &[u8], at: usize| read::<4>(bytes, at).map(u32::from_be_bytes);
    let be64 = |bytes: &[u8], at: usize| read::<8>(bytes, at).map(u64::from_be_bytes);

    let count = be32(bytes, 4)? as usize;
    let entry_size = if wide { 32 } else { 20 };
    let mut shared = 0;

    for index in 0..count {
        let entry = 8 + index * entry_size;

        let (offset, size) = if wide {
            (
                be64(bytes, entry + 8)? as usize,
                be64(bytes, entry + 16)? as usize,
            )
        } else {
            (
                be32(bytes, entry + 8)? as usize,
                be32(bytes, entry + 12)? as usize,
            )
        };

        let slice = offset
            .checked_add(size)
            .and_then(|end| bytes.get_mut(offset..end))
            .ok_or_else(|| format!("architecture {index} runs past the end of the library"))?;

        shared += share_in_library(slice)?;
    }

    Ok(shared)
}

fn share_in_archive(bytes: &mut [u8]) -> Result<usize, String> {
    let mut pos = ARCHIVE_MAGIC.len();
    let mut objects = 0;
    let mut shared = 0;

    while pos + 60 <= bytes.len() {
        let header = &bytes[pos..pos + 60];

        if &header[58..60] != b"`\n" {
            return Err(format!("malformed archive member header at byte {pos}"));
        }

        let size: usize = std::str::from_utf8(&header[48..58])
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .ok_or_else(|| format!("malformed archive member size at byte {pos}"))?;

        let name = String::from_utf8_lossy(&header[..16])
            .trim_end()
            .to_string();

        let start = pos + 60;
        let end = start
            .checked_add(size)
            .filter(|&end| end <= bytes.len())
            .ok_or_else(|| format!("archive member at byte {pos} runs past the end"))?;

        // A BSD archive (Apple's) keeps a long member name at the start of the member's data, as `#1/<length>`.
        let name_length = name
            .strip_prefix("#1/")
            .map(|n| n.parse::<usize>())
            .transpose()
            .map_err(|_| format!("malformed archive member name at byte {pos}"))?
            .unwrap_or(0)
            .min(size);

        let index = name == "/"
            || name == "//"
            || name == "/SYM64/"
            || name.starts_with("__.SYMDEF")
            || bytes[start..start + name_length].starts_with(b"__.SYMDEF");

        if !index {
            let member = &mut bytes[start + name_length..end];

            if let Some(changed) = share_in_object(member)? {
                objects += 1;
                shared += changed;
            }
        }

        pos = end + (size & 1);
    }

    if objects == 0 {
        return Err("the library holds no ELF, Mach-O or COFF object".into());
    }

    Ok(shared)
}

/// `None` when the member is no object file (an import entry of a Windows library, for one).
fn share_in_object(object: &mut [u8]) -> Result<Option<usize>, String> {
    if object.starts_with(b"\x7fELF") {
        return elf::weaken(object).map(Some);
    }

    if object.starts_with(&[0xcf, 0xfa, 0xed, 0xfe]) {
        return mach_o::weaken(object, true).map(Some);
    }

    if object.starts_with(&[0xce, 0xfa, 0xed, 0xfe]) {
        return mach_o::weaken(object, false).map(Some);
    }

    match coff::layout(object) {
        Some(layout) => coff::localize(object, layout).map(Some),
        None => Ok(None),
    }
}

fn read<const N: usize>(bytes: &[u8], at: usize) -> Result<[u8; N], String> {
    at.checked_add(N)
        .and_then(|end| bytes.get(at..end))
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| format!("object file truncated at byte {at}"))
}

fn u16_at(bytes: &[u8], at: usize) -> Result<u16, String> {
    read::<2>(bytes, at).map(u16::from_le_bytes)
}

fn u32_at(bytes: &[u8], at: usize) -> Result<u32, String> {
    read::<4>(bytes, at).map(u32::from_le_bytes)
}

fn u64_at(bytes: &[u8], at: usize) -> Result<usize, String> {
    read::<8>(bytes, at).map(|b| u64::from_le_bytes(b) as usize)
}

fn name_at(bytes: &[u8], at: usize) -> Result<&[u8], String> {
    let rest = bytes
        .get(at..)
        .ok_or_else(|| format!("symbol name out of bounds at byte {at}"))?;

    let len = rest
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| format!("unterminated symbol name at byte {at}"))?;

    Ok(&rest[..len])
}

mod elf {
    use super::*;

    pub const SHT_SYMTAB: u32 = 2;
    pub const STB_GLOBAL: u8 = 1;
    pub const STB_WEAK: u8 = 2;
    const SHN_UNDEF: u16 = 0;

    /// Offsets of the fields read, for 64-bit and 32-bit objects.
    struct Layout {
        wide: bool,
        section_table: usize,
        section_entry_size: usize,
        section_count: usize,
        section_type: usize,
        section_offset: usize,
        section_size: usize,
        section_link: usize,
        symbol_info: usize,
        symbol_section: usize,
        symbol_size: usize,
    }

    const LAYOUT_64: Layout = Layout {
        wide: true,
        section_table: 0x28,
        section_entry_size: 0x3a,
        section_count: 0x3c,
        section_type: 4,
        section_offset: 24,
        section_size: 32,
        section_link: 40,
        symbol_info: 4,
        symbol_section: 6,
        symbol_size: 24,
    };

    const LAYOUT_32: Layout = Layout {
        wide: false,
        section_table: 0x20,
        section_entry_size: 0x2e,
        section_count: 0x30,
        section_type: 4,
        section_offset: 16,
        section_size: 20,
        section_link: 24,
        symbol_info: 12,
        symbol_section: 14,
        symbol_size: 16,
    };

    pub fn weaken(object: &mut [u8]) -> Result<usize, String> {
        let layout = match object.get(4) {
            Some(2) => LAYOUT_64,
            Some(1) => LAYOUT_32,
            _ => return Err("ELF object of an unknown class".into()),
        };

        if object.get(5) != Some(&1) {
            return Err("big-endian ELF object: no platform of the engine uses one".into());
        }

        let word = |bytes: &[u8], at: usize| {
            if layout.wide {
                u64_at(bytes, at)
            } else {
                u32_at(bytes, at).map(|v| v as usize)
            }
        };

        let section_table = word(object, layout.section_table)?;
        let entry_size = u16_at(object, layout.section_entry_size)? as usize;
        let mut sections = u16_at(object, layout.section_count)? as usize;

        if sections == 0 && section_table != 0 {
            sections = word(object, section_table + layout.section_size)?;
        }

        let section = |index: usize| section_table + index * entry_size;
        let mut weakened = 0;

        for index in 0..sections {
            let header = section(index);

            if u32_at(object, header + layout.section_type)? != SHT_SYMTAB {
                continue;
            }

            let symbols = word(object, header + layout.section_offset)?;
            let size = word(object, header + layout.section_size)?;
            let names_section = section(u32_at(object, header + layout.section_link)? as usize);
            let names = word(object, names_section + layout.section_offset)?;

            for symbol in (symbols..symbols + size).step_by(layout.symbol_size) {
                let info = read::<1>(object, symbol + layout.symbol_info)?[0];
                let defined = u16_at(object, symbol + layout.symbol_section)? != SHN_UNDEF;

                if info >> 4 != STB_GLOBAL || !defined {
                    continue;
                }

                let name = name_at(object, names + u32_at(object, symbol)? as usize)?;

                if name == SHARED_SYMBOL.as_bytes() {
                    object[symbol + layout.symbol_info] = (STB_WEAK << 4) | (info & 0xf);
                    weakened += 1;
                }
            }
        }

        Ok(weakened)
    }
}

mod mach_o {
    use super::*;

    pub const LC_SYMTAB: u32 = 0x2;
    pub const N_STAB: u8 = 0xe0;
    pub const N_TYPE: u8 = 0x0e;
    pub const N_SECT: u8 = 0x0e;
    pub const N_EXT: u8 = 0x01;
    pub const N_WEAK_DEF: u16 = 0x0080;

    pub fn weaken(object: &mut [u8], wide: bool) -> Result<usize, String> {
        let header_size = if wide { 32 } else { 28 };
        let entry_size = if wide { 16 } else { 12 };
        let commands = u32_at(object, 16)? as usize;

        // Apple's symbols carry a leading underscore.
        let wanted = format!("_{SHARED_SYMBOL}");
        let mut command = header_size;
        let mut weakened = 0;

        for _ in 0..commands {
            let kind = u32_at(object, command)?;
            let size = u32_at(object, command + 4)? as usize;

            if kind == LC_SYMTAB {
                let symbols = u32_at(object, command + 8)? as usize;
                let count = u32_at(object, command + 12)? as usize;
                let names = u32_at(object, command + 16)? as usize;

                for index in 0..count {
                    let entry = symbols + index * entry_size;
                    let kind = read::<1>(object, entry + 4)?[0];
                    let description = u16_at(object, entry + 6)?;

                    let external_definition =
                        kind & N_STAB == 0 && kind & N_TYPE == N_SECT && kind & N_EXT != 0;

                    if !external_definition || description & N_WEAK_DEF != 0 {
                        continue;
                    }

                    let name = name_at(object, names + u32_at(object, entry)? as usize)?;

                    if name == wanted.as_bytes() {
                        let weak = description | N_WEAK_DEF;
                        object[entry + 6..entry + 8].copy_from_slice(&weak.to_le_bytes());
                        weakened += 1;
                    }
                }
            }

            if size == 0 {
                return Err("Mach-O load command of size 0".into());
            }

            command += size;
        }

        Ok(weakened)
    }
}

mod coff {
    use super::*;

    pub const IMAGE_SYM_CLASS_EXTERNAL: u8 = 2;
    pub const IMAGE_SYM_CLASS_STATIC: u8 = 3;

    const MACHINE_I386: u16 = 0x14c;
    const MACHINES: &[u16] = &[0x8664, 0xaa64, 0xa641, 0xa64e, 0x1c4, MACHINE_I386];

    /// The object a compiler writes when it has more than 65279 sections, as fat LTO may.
    const BIGOBJ_CLASS: [u8; 16] = [
        0xc7, 0xa1, 0xba, 0xd1, 0xee, 0xba, 0xa9, 0x4b, 0xaf, 0x20, 0xfa, 0xf6, 0x6a, 0xa4, 0xdc,
        0xb8,
    ];

    #[derive(Clone, Copy)]
    pub struct Layout {
        machine: u16,
        symbol_table: usize,
        symbol_count: usize,
        symbol_size: usize,
        section_number_wide: bool,
    }

    pub fn layout(object: &[u8]) -> Option<Layout> {
        let u16_at = |at| u16_at(object, at).ok();
        let u32_at = |at| u32_at(object, at).ok().map(|v| v as usize);

        let bigobj = u16_at(0)? == 0
            && u16_at(2)? == 0xffff
            && u16_at(4)? >= 2
            && object.get(12..28)? == BIGOBJ_CLASS;

        if bigobj {
            return Some(Layout {
                machine: u16_at(6)?,
                symbol_table: u32_at(48)?,
                symbol_count: u32_at(52)?,
                symbol_size: 20,
                section_number_wide: true,
            });
        }

        let machine = u16_at(0)?;

        // An object file has no optional header; an image has one.
        if !MACHINES.contains(&machine) || u16_at(16)? != 0 {
            return None;
        }

        Some(Layout {
            machine,
            symbol_table: u32_at(8)?,
            symbol_count: u32_at(12)?,
            symbol_size: 18,
            section_number_wide: false,
        })
    }

    pub fn localize(object: &mut [u8], layout: Layout) -> Result<usize, String> {
        // 32-bit x86 symbols carry a leading underscore.
        let wanted = if layout.machine == MACHINE_I386 {
            format!("_{SHARED_SYMBOL}")
        } else {
            SHARED_SYMBOL.to_string()
        };

        let names = layout.symbol_table + layout.symbol_count * layout.symbol_size;
        let class_at = if layout.section_number_wide { 18 } else { 16 };
        let mut localized = 0;
        let mut index = 0;

        while index < layout.symbol_count {
            let symbol = layout.symbol_table + index * layout.symbol_size;

            let section = if layout.section_number_wide {
                u32_at(object, symbol + 12)? as i32
            } else {
                u16_at(object, symbol + 12)? as i16 as i32
            };

            let class = read::<1>(object, symbol + class_at)?[0];
            let auxiliary = read::<1>(object, symbol + class_at + 1)?[0] as usize;

            if class == IMAGE_SYM_CLASS_EXTERNAL && section > 0 {
                let short = read::<8>(object, symbol)?;

                let name = if short[..4] == [0; 4] {
                    name_at(object, names + u32_at(object, symbol + 4)? as usize)?
                } else {
                    let len = short.iter().position(|&b| b == 0).unwrap_or(8);
                    &object[symbol..symbol + len]
                };

                if name == wanted.as_bytes() {
                    object[symbol + class_at] = IMAGE_SYM_CLASS_STATIC;
                    localized += 1;
                }
            }

            index += 1 + auxiliary;
        }

        Ok(localized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Symbol<'a> {
        name: &'a str,
        defined: bool,
    }

    const PERSONALITY: Symbol = Symbol {
        name: SHARED_SYMBOL,
        defined: true,
    };

    const ENTRY: Symbol = Symbol {
        name: "rusteal_entry",
        defined: true,
    };

    const REFERENCE: Symbol = Symbol {
        name: SHARED_SYMBOL,
        defined: false,
    };

    fn elf64(symbols: &[Symbol]) -> Vec<u8> {
        let mut names = vec![0u8];
        let mut table = vec![0u8; 24];

        for symbol in symbols {
            let offset = names.len() as u32;
            names.extend_from_slice(symbol.name.as_bytes());
            names.push(0);

            let mut entry = vec![0u8; 24];
            entry[0..4].copy_from_slice(&offset.to_le_bytes());
            entry[4] = (elf::STB_GLOBAL << 4) | 2;
            entry[6..8].copy_from_slice(&u16::from(symbol.defined).to_le_bytes());
            table.extend(entry);
        }

        let table_at = 64;
        let names_at = table_at + table.len();
        let sections_at = (names_at + names.len()).next_multiple_of(8);

        let mut out = vec![0u8; 64];
        out[0..4].copy_from_slice(b"\x7fELF");
        out[4] = 2;
        out[5] = 1;
        out[0x28..0x30].copy_from_slice(&(sections_at as u64).to_le_bytes());
        out[0x3a..0x3c].copy_from_slice(&64u16.to_le_bytes());
        out[0x3c..0x3e].copy_from_slice(&3u16.to_le_bytes());
        out.extend(&table);
        out.extend(&names);
        out.resize(sections_at, 0);

        let mut header = |kind: u32, at: usize, size: usize, link: u32, entry: u64| {
            let mut h = vec![0u8; 64];
            h[4..8].copy_from_slice(&kind.to_le_bytes());
            h[24..32].copy_from_slice(&(at as u64).to_le_bytes());
            h[32..40].copy_from_slice(&(size as u64).to_le_bytes());
            h[40..44].copy_from_slice(&link.to_le_bytes());
            h[56..64].copy_from_slice(&entry.to_le_bytes());
            out.extend(h);
        };

        header(0, 0, 0, 0, 0);
        header(elf::SHT_SYMTAB, table_at, table.len(), 2, 24);
        header(3, names_at, names.len(), 0, 0);
        out
    }

    fn elf64_bindings(object: &[u8]) -> Vec<u8> {
        let table = u64_at(object, 0x28).unwrap();
        let symbols = u64_at(object, table + 64 + 24).unwrap();
        let size = u64_at(object, table + 64 + 32).unwrap();
        (symbols + 24..symbols + size)
            .step_by(24)
            .map(|s| object[s + 4] >> 4)
            .collect()
    }

    fn mach_o64(symbols: &[Symbol]) -> Vec<u8> {
        let mut names = vec![b' ', 0];
        let mut table = Vec::new();

        for symbol in symbols {
            let offset = names.len() as u32;
            names.extend_from_slice(format!("_{}", symbol.name).as_bytes());
            names.push(0);

            let mut entry = vec![0u8; 16];
            entry[0..4].copy_from_slice(&offset.to_le_bytes());
            entry[4] = if symbol.defined {
                mach_o::N_SECT | mach_o::N_EXT
            } else {
                mach_o::N_EXT
            };
            entry[5] = u8::from(symbol.defined);
            table.extend(entry);
        }

        let table_at = 32 + 24;
        let names_at = table_at + table.len();

        let mut out = vec![0u8; 32];
        out[0..4].copy_from_slice(&[0xcf, 0xfa, 0xed, 0xfe]);
        out[12..16].copy_from_slice(&1u32.to_le_bytes());
        out[16..20].copy_from_slice(&1u32.to_le_bytes());
        out[20..24].copy_from_slice(&24u32.to_le_bytes());

        for field in [
            mach_o::LC_SYMTAB,
            24,
            table_at as u32,
            symbols.len() as u32,
            names_at as u32,
            names.len() as u32,
        ] {
            out.extend(field.to_le_bytes());
        }

        out.extend(&table);
        out.extend(&names);
        out
    }

    fn mach_o64_descriptions(object: &[u8]) -> Vec<u16> {
        let symbols = u32_at(object, 32 + 8).unwrap() as usize;
        let count = u32_at(object, 32 + 12).unwrap() as usize;
        (0..count)
            .map(|i| u16_at(object, symbols + i * 16 + 6).unwrap())
            .collect()
    }

    /// One auxiliary record follows the first symbol, as a section symbol has, to check that it is skipped.
    fn coff(symbols: &[Symbol], bigobj: bool) -> Vec<u8> {
        let symbol_size = if bigobj { 20 } else { 18 };
        let class_at = if bigobj { 18 } else { 16 };
        let header_size = if bigobj { 56 } else { 20 };
        let mut names = vec![0u8; 4];
        let mut table = Vec::new();
        let mut count = 0u32;

        for (i, symbol) in symbols.iter().enumerate() {
            let mut entry = vec![0u8; symbol_size];

            if symbol.name.len() <= 8 {
                entry[..symbol.name.len()].copy_from_slice(symbol.name.as_bytes());
            } else {
                entry[4..8].copy_from_slice(&(names.len() as u32).to_le_bytes());
                names.extend_from_slice(symbol.name.as_bytes());
                names.push(0);
            }

            entry[12] = u8::from(symbol.defined);
            entry[class_at] = coff::IMAGE_SYM_CLASS_EXTERNAL;
            entry[class_at + 1] = u8::from(i == 0);
            table.extend(entry);
            count += 1;

            if i == 0 {
                let mut auxiliary = vec![0u8; symbol_size];
                // Bytes that would read as an external definition of the personality, were the record not skipped.
                auxiliary[class_at] = coff::IMAGE_SYM_CLASS_EXTERNAL;
                auxiliary[12] = 1;
                table.extend(auxiliary);
                count += 1;
            }
        }

        let names_size = names.len() as u32;
        names[..4].copy_from_slice(&names_size.to_le_bytes());

        let mut out = vec![0u8; header_size];

        if bigobj {
            out[2..4].copy_from_slice(&0xffffu16.to_le_bytes());
            out[4..6].copy_from_slice(&2u16.to_le_bytes());
            out[6..8].copy_from_slice(&0x8664u16.to_le_bytes());
            out[12..28].copy_from_slice(&[
                0xc7, 0xa1, 0xba, 0xd1, 0xee, 0xba, 0xa9, 0x4b, 0xaf, 0x20, 0xfa, 0xf6, 0x6a, 0xa4,
                0xdc, 0xb8,
            ]);
            out[48..52].copy_from_slice(&(header_size as u32).to_le_bytes());
            out[52..56].copy_from_slice(&count.to_le_bytes());
        } else {
            out[0..2].copy_from_slice(&0x8664u16.to_le_bytes());
            out[8..12].copy_from_slice(&(header_size as u32).to_le_bytes());
            out[12..16].copy_from_slice(&count.to_le_bytes());
        }

        out.extend(&table);
        out.extend(&names);
        out
    }

    fn coff_classes(object: &[u8], bigobj: bool) -> Vec<u8> {
        let (symbol_size, class_at, table) = if bigobj { (20, 18, 56) } else { (18, 16, 20) };
        let count = u32_at(object, if bigobj { 52 } else { 12 }).unwrap() as usize;
        (0..count)
            .map(|i| object[table + i * symbol_size + class_at])
            .collect()
    }

    /// GNU-style member names, or BSD-style ones (`#1/<length>`, the name inside the data) as Apple's `ar` writes.
    fn archive(members: &[(&str, Vec<u8>)], bsd: bool) -> Vec<u8> {
        let mut out = ARCHIVE_MAGIC.to_vec();

        for (name, member) in members {
            let (header_name, data) = if bsd {
                let mut data = name.as_bytes().to_vec();
                data.extend(member);
                (format!("#1/{}", name.len()), data)
            } else {
                (format!("{name}/"), member.clone())
            };

            out.extend(
                format!(
                    "{:<16}{:<12}{:<6}{:<6}{:<8}{:<10}`\n",
                    header_name,
                    0,
                    0,
                    0,
                    644,
                    data.len()
                )
                .bytes(),
            );
            out.extend(&data);

            if data.len() % 2 == 1 {
                out.push(b'\n');
            }
        }

        out
    }

    fn members(bytes: &[u8]) -> Vec<&[u8]> {
        let mut found = Vec::new();
        let mut pos = ARCHIVE_MAGIC.len();

        while pos + 60 <= bytes.len() {
            let size: usize = std::str::from_utf8(&bytes[pos + 48..pos + 58])
                .unwrap()
                .trim()
                .parse()
                .unwrap();
            let name = std::str::from_utf8(&bytes[pos..pos + 16])
                .unwrap()
                .trim_end();
            let skip = name
                .strip_prefix("#1/")
                .map_or(0, |n| n.parse::<usize>().unwrap());
            found.push(&bytes[pos + 60 + skip..pos + 60 + size]);
            pos += 60 + size + (size & 1);
        }

        found
    }

    #[test]
    fn elf_the_personality_becomes_weak_and_nothing_else_changes() {
        let mut bytes = archive(
            &[
                ("a.o", elf64(&[ENTRY, PERSONALITY])),
                ("b.o", elf64(&[REFERENCE])),
            ],
            false,
        );

        assert_eq!(share_in_library(&mut bytes), Ok(1));

        let objects = members(&bytes);
        assert_eq!(elf64_bindings(objects[0]), [elf::STB_GLOBAL, elf::STB_WEAK]);
        assert_eq!(elf64_bindings(objects[1]), [elf::STB_GLOBAL]);
        assert_eq!(share_in_library(&mut bytes), Ok(0));
    }

    #[test]
    fn mach_o_the_personality_becomes_a_weak_definition() {
        let mut bytes = archive(
            &[
                ("__.SYMDEF SORTED", vec![0; 8]),
                ("a.o", mach_o64(&[ENTRY, PERSONALITY])),
                ("b.o", mach_o64(&[REFERENCE])),
            ],
            true,
        );

        assert_eq!(share_in_library(&mut bytes), Ok(1));

        let objects = members(&bytes);
        assert_eq!(mach_o64_descriptions(objects[1]), [0, mach_o::N_WEAK_DEF]);
        assert_eq!(mach_o64_descriptions(objects[2]), [0]);
        assert_eq!(share_in_library(&mut bytes), Ok(0));
    }

    #[test]
    fn mach_o_universal_library_changes_every_architecture() {
        let slice = archive(&[("a.o", mach_o64(&[PERSONALITY]))], true);
        let first = 4096usize;
        let second = first + slice.len().next_multiple_of(4096);

        let mut bytes = vec![0u8; second + slice.len()];
        bytes[0..4].copy_from_slice(&[0xca, 0xfe, 0xba, 0xbe]);
        bytes[4..8].copy_from_slice(&2u32.to_be_bytes());

        for (index, offset) in [first, second].into_iter().enumerate() {
            let entry = 8 + index * 20;
            bytes[entry + 8..entry + 12].copy_from_slice(&(offset as u32).to_be_bytes());
            bytes[entry + 12..entry + 16].copy_from_slice(&(slice.len() as u32).to_be_bytes());
            bytes[offset..offset + slice.len()].copy_from_slice(&slice);
        }

        assert_eq!(share_in_library(&mut bytes), Ok(2));
        assert_eq!(share_in_library(&mut bytes), Ok(0));
    }

    #[test]
    fn coff_the_personality_becomes_local() {
        for bigobj in [false, true] {
            let mut bytes = archive(
                &[
                    ("", vec![0; 8]),
                    ("", vec![0; 8]),
                    (
                        "a.obj",
                        coff(
                            &[
                                ENTRY,
                                PERSONALITY,
                                Symbol {
                                    name: "short",
                                    defined: true,
                                },
                            ],
                            bigobj,
                        ),
                    ),
                    ("b.obj", coff(&[REFERENCE], bigobj)),
                ],
                false,
            );

            assert_eq!(share_in_library(&mut bytes), Ok(1), "bigobj: {bigobj}");

            let objects = members(&bytes);
            let external = coff::IMAGE_SYM_CLASS_EXTERNAL;
            assert_eq!(
                coff_classes(objects[2], bigobj),
                [external, external, coff::IMAGE_SYM_CLASS_STATIC, external]
            );
            assert_eq!(coff_classes(objects[3], bigobj), [external, external]);
            assert_eq!(share_in_library(&mut bytes), Ok(0));
        }
    }

    #[test]
    fn a_file_that_is_no_static_library_is_refused() {
        assert!(share_in_library(&mut b"MZ not a library".to_vec()).is_err());
        assert!(share_in_library(&mut archive(&[("a.txt", b"text".to_vec())], false)).is_err());
    }
}
