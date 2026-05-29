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
            match opt.subsystem {
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
                ts => DateTime::<Utc>::from_timestamp(ts as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|_| "Invalid timestamp".to_string()),
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

fn extract_imports(pe: &PE) -> Vec<ImportDll> {
    let mut result = Vec::new();
    if let Some(imports) = &pe.imports {
        for imp in imports {
            let dll_name = imp.dll.to_string();
            let functions: Vec<String> = imp.symbols.iter().map(|sym| match sym {
                goblin::pe::import::Symbol::ImportByName(name) => name.to_string(),
                goblin::pe::import::Symbol::ImportByOrdinal(ord) => format!("Ordinal({})", ord),
            }).collect();
            result.push(ImportDll { name: dll_name, functions });
        }
    }
    result
}

fn extract_strings(data: &[u8]) -> (Vec<String>, Vec<String>) {
    let mut ascii = Vec::new();
    let mut current = String::new();
    for &b in data {
        if b >= 0x20 && b <= 0x7E {
            current.push(b as char);
        } else {
            if current.len() >= 4 {
                ascii.push(current.clone());
            }
            current.clear();
        }
    }
    if current.len() >= 4 {
        ascii.push(current);
    }

    let mut unicode = Vec::new();
    let mut current_u = String::new();
    let mut i = 0;
    while i + 1 < data.len() {
        let b0 = data[i];
        let b1 = data[i + 1];
        if b1 == 0 && b0 >= 0x20 && b0 <= 0x7E {
            current_u.push(b0 as char);
            i += 2;
        } else {
            if current_u.len() >= 4 {
                unicode.push(current_u.clone());
            }
            current_u.clear();
            i += 1;
        }
    }
    if current_u.len() >= 4 {
        unicode.push(current_u);
    }

    let keywords = [
        "psexec", "sysinternals", "microsoft", "service", "cmd.exe",
        "powershell", "registry", "wow64", "pipe", "ntdll", "kernel32",
        "advapi", "advapi32", "token", "process", "thread", "create", "open",
        "shellexecute", "crypt", "lsa", "logon", "scmanager", "namedpipe",
    ];

    let ascii_filtered: Vec<_> = ascii.into_iter()
        .filter(|s| {
            let lower = s.to_lowercase();
            s.len() >= 6 && keywords.iter().any(|&k| lower.contains(k))
        })
        .collect();

    let unicode_filtered: Vec<_> = unicode.into_iter()
        .filter(|s| {
            let lower = s.to_lowercase();
            s.len() >= 6 && keywords.iter().any(|&k| lower.contains(k))
        })
        .collect();

    (ascii_filtered, unicode_filtered)
}
