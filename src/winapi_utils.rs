use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::core::PCWSTR;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Security::WinTrust::{
    WinVerifyTrust, WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_DATA_0,
    WINTRUST_FILE_INFO, WTD_CHOICE_FILE, WTD_REVOKE_NONE, WTD_STATEACTION_VERIFY,
    WTD_UI_NONE, WTD_STATEACTION_CLOSE,
};
use windows::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
};

use crate::analyzer::{SignatureInfo, VersionInfo};

pub fn get_version_info(path: &Path) -> VersionInfo {
    let mut vi = VersionInfo::default();
    let wide: Vec<u16> = OsStr::new(path).encode_wide().chain(Some(0)).collect();

    unsafe {
        let mut handle = 0u32;
        let size = GetFileVersionInfoSizeW(PCWSTR(wide.as_ptr()), Some(&mut handle));
        if size == 0 {
            return vi;
        }

        let mut buffer = vec![0u8; size as usize];
        if GetFileVersionInfoW(
            PCWSTR(wide.as_ptr()),
            handle,
            size,
            buffer.as_mut_ptr() as *mut _,
        ).is_err() {
            return vi;
        }

        let mut lang_ptr = std::ptr::null_mut();
        let mut lang_len = 0u32;
        if VerQueryValueW(
            buffer.as_ptr() as *const _,
            windows::core::w!("\\VarFileInfo\\Translation"),
            &mut lang_ptr,
            &mut lang_len,
        ).as_bool() && lang_len >= 4
        {
            let lang = *(lang_ptr as *const u16);
            let cp = *((lang_ptr as *const u16).add(1));
            let lang_str = format!("{:04x}{:04x}", lang, cp);

            vi.file_version = query_value(&buffer, &format!("\\StringFileInfo\\{}\\FileVersion", lang_str));
            vi.product_version = query_value(&buffer, &format!("\\StringFileInfo\\{}\\ProductVersion", lang_str));
            vi.company_name = query_value(&buffer, &format!("\\StringFileInfo\\{}\\CompanyName", lang_str));
            vi.file_description = query_value(&buffer, &format!("\\StringFileInfo\\{}\\FileDescription", lang_str));
            vi.product_name = query_value(&buffer, &format!("\\StringFileInfo\\{}\\ProductName", lang_str));
            vi.original_filename = query_value(&buffer, &format!("\\StringFileInfo\\{}\\OriginalFilename", lang_str));
            vi.internal_name = query_value(&buffer, &format!("\\StringFileInfo\\{}\\InternalName", lang_str));
            vi.copyright = query_value(&buffer, &format!("\\StringFileInfo\\{}\\LegalCopyright", lang_str));
        }
    }

    vi
}

unsafe fn query_value(buffer: &[u8], key: &str) -> String {
    let wide: Vec<u16> = OsStr::new(key).encode_wide().chain(Some(0)).collect();
    let mut ptr = std::ptr::null_mut();
    let mut len = 0u32;
    if VerQueryValueW(
        buffer.as_ptr() as *const _,
        PCWSTR(wide.as_ptr()),
        &mut ptr,
        &mut len,
    ).as_bool() && len > 0
    {
        // SAFETY: Check that ptr is not null before dereferencing
        if ptr.is_null() {
            return String::new();
        }

        // SAFETY: VerQueryValueW returns byte count; convert to u16 count
        let u16_count = (len as usize) / std::mem::size_of::<u16>();

        // SAFETY: Ensure we have at least 1 u16 element
        if u16_count == 0 {
            return String::new();
        }

        // SAFETY: ptr comes from VerQueryValueW and is guaranteed to be within buffer
        // if successful. We've validated it's not null and has at least 1 element.
        let slice = std::slice::from_raw_parts(ptr as *const u16, u16_count);
        String::from_utf16_lossy(slice).trim_end_matches('\0').to_string()
    } else {
        String::new()
    }
}

pub fn verify_signature(path: &Path) -> SignatureInfo {
    let mut sig = SignatureInfo::default();
    let wide: Vec<u16> = OsStr::new(path).encode_wide().chain(Some(0)).collect();

    unsafe {
        let file_info = WINTRUST_FILE_INFO {
            cbStruct: std::mem::size_of::<WINTRUST_FILE_INFO>() as u32,
            pcwszFilePath: PCWSTR(wide.as_ptr()),
            hFile: HANDLE(0),
            pgKnownSubject: std::ptr::null_mut(),
        };

        let mut data = WINTRUST_DATA {
            cbStruct: std::mem::size_of::<WINTRUST_DATA>() as u32,
            pPolicyCallbackData: std::ptr::null_mut(),
            pSIPClientData: std::ptr::null_mut(),
            dwUIChoice: WTD_UI_NONE,
            fdwRevocationChecks: WTD_REVOKE_NONE,
            dwUnionChoice: WTD_CHOICE_FILE,
            Anonymous: WINTRUST_DATA_0 {
                pFile: &file_info as *const _ as *mut _,
            },
            dwStateAction: WTD_STATEACTION_VERIFY,
            hWVTStateData: HANDLE(0),
            pwszURLReference: windows::core::PWSTR(std::ptr::null_mut()),
            dwProvFlags: windows::Win32::Security::WinTrust::WINTRUST_DATA_PROVIDER_FLAGS(0),
            dwUIContext: windows::Win32::Security::WinTrust::WINTRUST_DATA_UICONTEXT(0),
            pSignatureSettings: std::ptr::null_mut(),
        };

        let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
        let hr = WinVerifyTrust(
            windows::Win32::Foundation::HWND(0),
            &mut action as *mut _,
            &mut data as *mut _ as *mut _,
        );

        if hr == 0 {
            sig.status = "Valid".to_string();
        } else {
            sig.status = format!("Invalid / Untrusted (Error: {})", hr);
        }

        data.dwStateAction = WTD_STATEACTION_CLOSE;
        let _ = WinVerifyTrust(
            windows::Win32::Foundation::HWND(0),
            &mut action as *mut _,
            &mut data as *mut _ as *mut _,
        );
    }

    sig
}
