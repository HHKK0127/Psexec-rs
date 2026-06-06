use windows::core::PCWSTR;
use windows::Win32::Foundation::{BOOL, HANDLE, CloseHandle};
use windows::Win32::System::Threading::*;
use windows::Win32::Security::*;
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::System::Pipes::ImpersonateNamedPipeClient;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::settings::RemoteSettings;

pub struct ProcessInfo {
    pub process_handle: HANDLE,
    pub thread_handle: HANDLE,
    pub process_id: u32,
}

pub fn start_process(
    settings: &RemoteSettings,
    cmd_pipe: Option<HANDLE>,
) -> Result<ProcessInfo, String> {
    let user_handle = get_user_handle(settings, cmd_pipe)?;

    let app_path = format!("\"{}\"", settings.app);
    let cmd_line = if settings.app_args.is_empty() {
        app_path.clone()
    } else {
        format!("{} {}", app_path, settings.app_args)
    };

    let mut si = STARTUPINFOW {
        cb: std::mem::size_of::<STARTUPINFOW>() as u32,
        dwFlags: STARTF_USESHOWWINDOW,
        wShowWindow: if settings.interactive || settings.show_ui_on_winlogon {
            SW_SHOW
        } else {
            SW_HIDE
        },
        hStdInput: settings.h_stdin,
        hStdOutput: settings.h_stdout,
        hStdError: settings.h_stderr,
        lpDesktop: if settings.show_ui_on_winlogon {
            PCWSTR::from_raw(encode_wide("WinSta0\\Winlogon").as_ptr())
        } else if settings.interactive {
            PCWSTR::from_raw(encode_wide("Winsta0\\default").as_ptr())
        } else {
            PCWSTR::null()
        },
        ..Default::default()
    };

    if !is_bad_handle(settings.h_stdout) {
        si.dwFlags |= STARTF_USESTDHANDLES;
    }

    let wd_opt = if !settings.working_dir.is_empty() {
        Some(encode_wide(&settings.working_dir))
    } else {
        None
    };

    let cmd_wide: Vec<u16> = encode_wide(&cmd_line);

    let mut pi = PROCESS_INFORMATION::default();

    let dw_flags = CREATE_SUSPENDED | CREATE_NEW_CONSOLE | CREATE_UNICODE_ENVIRONMENT;

    let launch_result = unsafe {
        CreateProcessAsUserW(
            user_handle,
            None,
            PCWSTR::from_raw(cmd_wide.as_ptr()),
            None,
            None,
            true,
            dw_flags,
            None,
            wd_opt.as_ref().map(|w| w.as_ptr() as *const u16),
            &si,
            &mut pi,
        )
    };

    let gle = unsafe { GetLastError() };

    if launch_result.is_err() {
        return Err(format!(
            "CreateProcessAsUserW failed: error {}",
            gle.0
        ));
    }

    if !settings.allowed_processors.is_empty() {
        let mut sys_mask = 0usize;
        let mut proc_mask = 0usize;
        unsafe {
            let _ = GetProcessAffinityMask(
                pi.hProcess,
                Some(&mut proc_mask),
                Some(&mut sys_mask),
            );
            let mut new_mask = 0usize;
            for &cpu in &settings.allowed_processors {
                new_mask |= 1usize << (cpu as usize - 1);
            }
            new_mask &= sys_mask;
            if new_mask != 0 {
                let _ = SetProcessAffinityMask(pi.hProcess, new_mask);
            }
        }
    }

    unsafe {
        let _ = SetPriorityClass(pi.hProcess, settings.priority);
        ResumeThread(pi.hThread);
    }

    Ok(ProcessInfo {
        process_handle: pi.hProcess,
        thread_handle: pi.hThread,
        process_id: pi.dwProcessId,
    })
}

fn get_user_handle(
    settings: &RemoteSettings,
    cmd_pipe: Option<HANDLE>,
) -> Result<HANDLE, String> {
    unsafe {
        if settings.use_system_account {
            get_local_system_token()
        } else if !settings.user.is_empty() {
            logon_user(&settings.user, &settings.password)
        } else {
            get_current_user_token(cmd_pipe)
        }
    }
}

unsafe fn logon_user(user: &str, password: &str) -> Result<HANDLE, String> {
    let (username, domain) = split_user_domain(user);

    let user_wide = encode_wide(&username);
    let domain_wide = encode_wide(&domain);
    let pass_wide = encode_wide(password);

    let mut token = HANDLE::default();
    let result = LogonUserW(
        PCWSTR::from_raw(user_wide.as_ptr()),
        if domain.is_empty() {
            PCWSTR::null()
        } else {
            PCWSTR::from_raw(domain_wide.as_ptr())
        },
        PCWSTR::from_raw(pass_wide.as_ptr()),
        LOGON32_LOGON_INTERACTIVE,
        LOGON32_PROVIDER_DEFAULT,
        &mut token,
    );

    if result.is_err() {
        return Err(format!("LogonUser failed for {}", user));
    }

    let token = duplicate_token_max(token)?;
    Ok(token)
}

unsafe fn get_local_system_token() -> Result<HANDLE, String> {
    use windows::Win32::System::ProcessStatus::*;
    use windows::Win32::Security::*;

    let mut pids = [0u32; 10240];
    let mut needed = 0u32;

    if EnumProcesses(
        &mut pids,
        std::mem::size_of_val(&pids) as u32,
        &mut needed,
    )
    .is_err()
    {
        return Err("EnumProcesses failed".into());
    }

    let count = (needed / 4) as usize;
    for i in 0..count {
        let pid = pids[i];
        if pid == 0 {
            continue;
        }

        let proc_handle = OpenProcess(
            PROCESS_QUERY_INFORMATION,
            false,
            pid,
        );

        let proc_handle = match proc_handle {
            Ok(h) => h,
            Err(_) => continue,
        };

        let mut token = HANDLE::default();
        if OpenProcessToken(
            proc_handle,
            TOKEN_QUERY | TOKEN_DUPLICATE | TOKEN_ASSIGN_PRIMARY,
            &mut token,
        )
        .is_ok()
        {
            let sid = get_token_user_sid(token);
            if sid == "S-1-5-18" {
                let _ = CloseHandle(proc_handle);
                return duplicate_token_max(token);
            }
            let _ = CloseHandle(token);
        }
        let _ = CloseHandle(proc_handle);
    }

    Err("Could not find SYSTEM token".into())
}

unsafe fn get_current_user_token(cmd_pipe: Option<HANDLE>) -> Result<HANDLE, String> {
    if let Some(pipe) = cmd_pipe {
        let _ = ImpersonateNamedPipeClient(pipe);
    }

    let thread = GetCurrentThread();
    let mut token = HANDLE::default();

    let dup_result = DuplicateHandle(
        GetCurrentProcess(),
        thread,
        GetCurrentProcess(),
        &mut token,
        0,
        true,
        DUPLICATE_SAME_ACCESS,
    );

    if dup_result.is_err() || OpenThreadToken(thread, TOKEN_DUPLICATE | TOKEN_QUERY, true, &mut token).is_err()
    {
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_DUPLICATE | TOKEN_QUERY,
            &mut token,
        )
        .map_err(|_| "Failed to get current user token".to_string())?;
    }

    let token = duplicate_token_max(token)?;
    let _ = RevertToSelf();
    Ok(token)
}

unsafe fn duplicate_token_max(token: HANDLE) -> Result<HANDLE, String> {
    let mut dup = HANDLE::default();
    if DuplicateTokenEx(
        token,
        TOKEN_ASSIGN_PRIMARY | TOKEN_DUPLICATE | TOKEN_QUERY,
        None,
        SecurityImpersonation,
        TokenPrimary,
        &mut dup,
    )
    .is_err()
    {
        return Err("DuplicateTokenEx failed".into());
    }
    let _ = CloseHandle(token);
    Ok(dup)
}

unsafe fn get_token_user_sid(token: HANDLE) -> String {
    use windows::Win32::Security::*;
    let mut size = 0u32;
    let _ = GetTokenInformation(token, TokenUser, None, 0, &mut size);

    let mut buf = vec![0u8; size as usize];
    if GetTokenInformation(
        token,
        TokenUser,
        Some(buf.as_mut_ptr() as *mut _),
        size,
        &mut size,
    )
    .is_err()
    {
        return String::new();
    }

    let token_user = buf.as_ptr() as *const TOKEN_USER;
    let sid = (*token_user).User.Sid;

    let mut sid_str = windows::core::PWSTR::default();
    if ConvertSidToStringSidW(sid, &mut sid_str).is_ok() {
        let s = sid_str.to_string().unwrap_or_default();
        let _ = LocalFree(sid_str.as_ptr() as _);
        s
    } else {
        String::new()
    }
}

fn split_user_domain(user: &str) -> (String, String) {
    if let Some(pos) = user.find('\\') {
        let domain = user[..pos].to_string();
        let username = user[pos + 1..].to_string();
        (username, domain)
    } else if let Some(pos) = user.find('@') {
        let username = user[..pos].to_string();
        let domain = user[pos + 1..].to_string();
        (username, domain)
    } else {
        (user.to_string(), String::new())
    }
}

fn encode_wide(s: &str) -> Vec<u16> {
    std::os::windows::ffi::OsStr::new(s)
        .encode_wide()
        .chain(Some(0))
        .collect()
}

fn is_bad_handle(h: HANDLE) -> bool {
    h.is_invalid() || h == HANDLE(0)
}
