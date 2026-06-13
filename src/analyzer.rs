use goblin::pe::PE;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use chrono::{DateTime, Local, Utc};

const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;

#[derive(Default, Clone, Debug)]
pub struct AnalysisResult {
    pub file_path: String,
    pub file_name: String,
    pub file_size: u64,
    pub created: String,
    pub modified: String,
    pub sha256: String,
    pub pe_info: PeInfo,
    pub version_info: VersionInfo,
    pub signature: SignatureInfo,
    pub imports: Vec<ImportDll>,
    pub strings_ascii: Vec<String>,
    pub strings_unicode: Vec<String>,
}

#[derive(Default, Clone, Debug)]
pub struct PeInfo {
    pub is_64bit: bool,
    pub machine: String,
    pub subsystem: String,
    pub entry_point: String,
    pub image_base: String,
    pub timestamp: String,
    pub sections: Vec<String>,
}

#[derive(Default, Clone, Debug)]
pub struct VersionInfo {
    pub file_version: String,
    pub product_version: String,
    pub company_name: String,
    pub file_description: String,
    pub product_name: String,
    pub original_filename: String,
    pub internal_name: String,
    pub copyright: String,
}

#[derive(Default, Clone, Debug)]
pub struct SignatureInfo {
    pub status: String,
    pub signer: String,
    pub serial: String,
    pub thumbprint: String,
    pub valid_from: String,
    pub valid_to: String,
}

#[derive(Default, Clone, Debug)]
pub struct ImportDll {
    pub name: String,
    pub functions: Vec<String>,
}

pub fn analyze_file(path: &Path) -> Result<AnalysisResult, String> {
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    if metadata.len() > MAX_FILE_SIZE {
        return Err(format!(
            "File too large: {} MB (max: {} MB)",
            metadata.len() / 1024 / 1024,
            MAX_FILE_SIZE / 1024 / 1024
        ));
    }

    let data = fs::read(path).map_err(|e| e.to_string())?;
    let size = data.len() as u64;

    let created = metadata.created().ok()
        .map(|t| DateTime::<Local>::from(t).to_string());
    let modified = metadata.modified().ok()
        .map(|t| DateTime::<Local>::from(t).to_string());

    let sha256 = compute_sha256(&data);

    let pe = match PE::parse(&data) {
        Ok(pe) => pe,
        Err(e) => return Err(format!("PE parse error: {}", e)),
    };

    let pe_info = extract_pe_info(&pe);
    let imports = extract_imports(&pe);
    let (strings_ascii, strings_unicode) = extract_strings(&data);
    let version_info = crate::winapi_utils::get_version_info(path);
    let signature = crate::winapi_utils::verify_signature(path);

    Ok(AnalysisResult {
        file_path: path.to_string_lossy().to_string(),
        file_name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
        file_size: size,
        created: created.unwrap_or_default(),
        modified: modified.unwrap_or_default(),
        sha256,
        pe_info,
        version_info,
        signature,
        imports,
        strings_ascii,
        strings_unicode,
    })
}

fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn extract_pe_info(pe: &PE) -> PeInfo {
    let is_64bit = pe.is_64;
    let machine = match pe.header.coff_header.machine {
        0x014c => "i386 (x86)".to_string(),
        0x8664 => "AMD64 (x64)".to_string(),
        0xaa64 => "ARM64".to_string(),
        0x01c0 => "ARM".to_string(),
        m => format!("Unknown (0x{:04X})", m),
    };
    let (subsystem, entry_point, image_base, timestamp) = match pe.header.optional_header {
        Some(opt) => (
            match opt.windows_fields.subsystem {
                1 => "Native (Driver)".to_string(),
                2 => "Windows GUI".to_string(),
                3 => "Windows Console (CUI)".to_string(),
                5 => "OS/2 Console".to_string(),
                7 => "POSIX Console".to_string(),
                9 => "Windows CE GUI".to_string(),
                10 => "EFI Application".to_string(),
                11 => "EFI Boot Driver".to_string(),
                12 => "EFI Runtime Driver".to_string(),
                13 => "EFI ROM".to_string(),
                14 => "Xbox".to_string(),
                16 => "Windows Boot App".to_string(),
                s => format!("Unknown (0x{:04X})", s),
            },
            format!("0x{:08X}", opt.standard_fields.address_of_entry_point),
            if is_64bit {
                format!("0x{:016X}", opt.windows_fields.image_base)
            } else {
                format!("0x{:08X}", opt.windows_fields.image_base as u32)
            },
            match pe.header.coff_header.time_date_stamp {
                0 => "Unknown".to_string(),
                ts => match DateTime::<Utc>::from_timestamp(ts as i64, 0) {
                    Some(dt) => dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                    None => "Invalid timestamp".to_string(),
                },
            },
        ),
        None => (
            "N/A".into(),
            "N/A".into(),
            "N/A".into(),
            "N/A".into(),
        ),
    };
    let sections = pe.sections.iter().map(|s| {
        let name = std::str::from_utf8(&s.name)
            .unwrap_or("")
            .trim_end_matches('\0')
            .to_string();
        format!(
            "{} (RVA: 0x{:08X}, VirtualSize: 0x{:08X}, RawSize: 0x{:08X})",
            name, s.virtual_address, s.virtual_size, s.size_of_raw_data
        )
    }).collect();

    PeInfo {
        is_64bit,
        machine,
        subsystem,
        entry_point,
        image_base,
        timestamp,
        sections,
    }
}

/// Fixed for goblin v0.8 API: pe.imports is Vec<Import>, not Option<Vec<Import>>
/// Each Import represents a single symbol imported from a DLL
/// In goblin v0.8, imports are flattened - each import has dll name and one symbol
fn extract_imports(pe: &PE) -> Vec<ImportDll> {
    use std::collections::HashMap;

    // Group imports by DLL name
    let mut dll_map: HashMap<String, Vec<String>> = HashMap::new();

    // goblin v0.8: pe.imports is Vec<Import>, each has dll and symbol name
    for imp in &pe.imports {
        let dll_name = imp.dll.to_string();
        let func_name = imp.name.to_string();

        dll_map.entry(dll_name)
            .or_insert_with(Vec::new)
            .push(func_name);
    }

    // Convert HashMap to Vec<ImportDll>
    let mut result: Vec<ImportDll> = dll_map
        .into_iter()
        .map(|(name, functions)| ImportDll { name, functions })
        .collect();

    result.sort_by(|a, b| a.name.cmp(&b.name));
    result
}

/// Extract strings from binary data with keyword filtering.
///
/// This function extracts ASCII and Unicode (UTF-16LE) strings from PE binaries,
/// filtering to only return strings that match PAExec-relevant keywords.
///
/// **Note**: This filters by hardcoded keywords (psexec, service, process, token, etc.).
/// Users see only filtered results; the full string count is not displayed.
fn extract_strings(data: &[u8]) -> (Vec<String>, Vec<String>) {
    // Extract ASCII strings (0x20-0x7E printable range)
    let ascii = extract_ascii_strings(data);

    // Extract Unicode strings (UTF-16LE format)
    let unicode = extract_unicode_strings(data);

    // Filter by hardcoded keywords
    let keywords = [
        "psexec", "sysinternals", "microsoft", "service", "cmd.exe",
        "powershell", "registry", "wow64", "pipe", "ntdll", "kernel32",
        "advapi", "advapi32", "token", "process", "thread", "create", "open",
        "shellexecute", "crypt", "lsa", "logon", "scmanager", "namedpipe",
    ];

    let ascii_filtered: Vec<_> = ascii
        .into_iter()
        .filter(|s| {
            let lower = s.to_lowercase();
            s.len() >= 6 && keywords.iter().any(|&k| lower.contains(k))
        })
        .collect();

    let unicode_filtered: Vec<_> = unicode
        .into_iter()
        .filter(|s| {
            let lower = s.to_lowercase();
            s.len() >= 6 && keywords.iter().any(|&k| lower.contains(k))
        })
        .collect();

    (ascii_filtered, unicode_filtered)
}

/// Extract ASCII strings (0x20-0x7E range) from binary data.
pub fn extract_ascii_strings(data: &[u8]) -> Vec<String> {
    let mut strings = Vec::new();
    let mut current = String::new();

    for &b in data {
        if b >= 0x20 && b <= 0x7E {
            current.push(b as char);
        } else {
            if current.len() >= 4 {
                strings.push(current.clone());
            }
            current.clear();
        }
    }

    if current.len() >= 4 {
        strings.push(current);
    }

    strings
}

/// Extract Unicode strings (UTF-16LE format) from binary data.
///
/// Properly handles UTF-16LE encoded strings by:
/// 1. Reading pairs of bytes as u16 (little-endian)
/// 2. Validating characters (excluding surrogates, null terminators)
/// 3. Always incrementing by 2 bytes (not 1), avoiding overlapping scans
pub fn extract_unicode_strings(data: &[u8]) -> Vec<String> {
    let mut strings = Vec::new();
    let mut current = String::new();
    let mut i = 0;

    while i + 1 < data.len() {
        // Read u16 in little-endian format
        let u16_char = u16::from_le_bytes([data[i], data[i + 1]]);

        // Check if this is a valid UTF-16 code point
        // Exclude: null terminators (0x0000), surrogates (0xD800-0xDFFF)
        if u16_char == 0 {
            // Null terminator: end of string
            if current.len() >= 4 {
                strings.push(current.clone());
            }
            current.clear();
        } else if u16_char >= 0xD800 && u16_char <= 0xDFFF {
            // Surrogate pair (not handled in this simple version)
            // End current string and skip
            if current.len() >= 4 {
                strings.push(current.clone());
            }
            current.clear();
        } else if let Some(ch) = char::from_u32(u16_char as u32) {
            // Valid Unicode character
            if ch.is_ascii_graphic() || ch == ' ' || ch == '\t' || ch == '\n' {
                current.push(ch);
            } else {
                // Non-printable: end string
                if current.len() >= 4 {
                    strings.push(current.clone());
                }
                current.clear();
            }
        }

        i += 2; // Always increment by 2 for UTF-16
    }

    // Flush any remaining string
    if current.len() >= 4 {
        strings.push(current);
    }

    strings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_sha256() {
        let data = b"hello world";
        let hash = compute_sha256(data);
        assert_eq!(hash, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
    }

    #[test]
    fn test_compute_sha256_empty() {
        let data = b"";
        let hash = compute_sha256(data);
        assert_eq!(hash, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    }

    #[test]
    fn test_extract_ascii_strings_basic() {
        let data = b"hello\x00world\x00test";
        let result = extract_ascii_strings(data);
        assert!(result.contains(&"hello".to_string()));
        assert!(result.contains(&"world".to_string()));
        assert!(result.contains(&"test".to_string()));
    }

    #[test]
    fn test_extract_ascii_strings_min_length() {
        let data = b"abc\x00defg\x00toolong";
        let result = extract_ascii_strings(data);
        assert!(!result.contains(&"abc".to_string()));
        assert!(result.contains(&"defg".to_string()));
        assert!(result.contains(&"toolong".to_string()));
    }

    #[test]
    fn test_extract_ascii_strings_empty() {
        let data = b"";
        let result = extract_ascii_strings(data);
        assert!(result.is_empty());
    }

    #[test]
    fn test_extract_unicode_strings_basic() {
        let data = "hello".encode_utf16(|c| {
            let bytes = (c as u16).to_le_bytes();
            [bytes[0], bytes[1]]
        }).flatten().collect::<Vec<u8>>();
        let result = extract_unicode_strings(&data);
        assert!(result.iter().any(|s| s.contains("hello")));
    }

    #[test]
    fn test_extract_unicode_strings_empty() {
        let data: Vec<u8> = vec![];
        let result = extract_unicode_strings(&data);
        assert!(result.is_empty());
    }

    #[test]
    fn test_analysis_result_default() {
        let result = AnalysisResult::default();
        assert_eq!(result.file_path, "");
        assert_eq!(result.file_size, 0);
        assert_eq!(result.sha256, "");
    }
}
