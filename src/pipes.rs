use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::os::windows::ffi::OsStrExt;
use windows::Win32::Foundation::{HANDLE, BOOL};
use windows::Win32::System::Pipes::*;
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::System::Console::*;
use windows::Win32::Security::SECURITY_ATTRIBUTES;
use windows::core::PCWSTR;

use crate::settings::RemoteSettings;

pub static STOP_FLAG: AtomicBool = AtomicBool::new(false);

pub struct IoPipes {
    pub stdout_pipe: HANDLE,
    pub stderr_pipe: HANDLE,
    pub stdin_pipe: HANDLE,
}

const PIPE_BUFFER_SIZE: usize = 4096;

pub fn create_io_pipes_in_service(
    settings: &mut RemoteSettings,
    caller: &str,
    pid: u32,
) -> Result<IoPipes, String> {
    let mut sec_desc = build_secure_pipe_security_descriptor()?;

    let stdout_name = format!(r"\\.\pipe\PAExecOut{}{}", caller, pid);
    let stderr_name = format!(r"\\.\pipe\PAExecErr{}{}", caller, pid);
    let stdin_name = format!(r"\\.\pipe\PAExecIn{}{}", caller, pid);

    let mut sa = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: sec_desc.as_mut_ptr() as *mut std::ffi::c_void,
        bInheritHandle: BOOL(0),
    };

    fn create_pipe(name: &str, outbound: bool, sa: &SECURITY_ATTRIBUTES) -> Result<HANDLE, String> {
        let wide: Vec<u16> = std::os::windows::ffi::OsStr::new(name)
            .encode_wide()
            .chain(Some(0))
            .collect();

        unsafe {
            let handle = CreateNamedPipeW(
                PCWSTR(wide.as_ptr()),
                if outbound {
                    PIPE_ACCESS_OUTBOUND
                } else {
                    PIPE_ACCESS_INBOUND
                },
                PIPE_TYPE_BYTE | PIPE_WAIT,
                PIPE_UNLIMITED_INSTANCES,
                0,
                0,
                0xFFFFFFFF,
                Some(sa as *const SECURITY_ATTRIBUTES),
            )
            .map_err(|e| format!("CreateNamedPipe({}) failed: {}", name, e))?;

            Ok(handle)
        }
    }

    let stdout_pipe = create_pipe(&stdout_name, true, &sa)?;
    let stderr_pipe = create_pipe(&stderr_name, true, &sa)?;
    let stdin_pipe = create_pipe(&stdin_name, false, &sa)?;

    unsafe {
        ConnectNamedPipe(stdout_pipe, None).ok();
        ConnectNamedPipe(stderr_pipe, None).ok();
        ConnectNamedPipe(stdin_pipe, None).ok();
    }

    settings.h_stdout = stdout_pipe;
    settings.h_stderr = stderr_pipe;
    settings.h_stdin = stdin_pipe;

    Ok(IoPipes {
        stdout_pipe,
        stderr_pipe,
        stdin_pipe,
    })
}

pub fn connect_to_remote_pipes(
    server: &str,
    machine_name: &str,
    pid: u32,
    settings: &mut RemoteSettings,
    stop: Arc<AtomicBool>,
    retries: u32,
) -> bool {
    let stdout_name = format!(r"\\{}\pipe\PAExecOut{}{}", server, machine_name, pid);
    let stderr_name = format!(r"\\{}\pipe\PAExecErr{}{}", server, machine_name, pid);
    let stdin_name = format!(r"\\{}\pipe\PAExecIn{}{}", server, machine_name, pid);

    for attempt in 0..retries {
        if stop.load(Ordering::Relaxed) {
            return false;
        }

        if is_bad_handle(settings.h_stdout) {
            if let Ok(h) = connect_to_pipe(&stdout_name, GENERIC_READ) {
                settings.h_stdout = h;
            }
        }
        if is_bad_handle(settings.h_stderr) {
            if let Ok(h) = connect_to_pipe(&stderr_name, GENERIC_READ) {
                settings.h_stderr = h;
            }
        }
        if is_bad_handle(settings.h_stdin) {
            if let Ok(h) = connect_to_pipe(&stdin_name, GENERIC_WRITE) {
                settings.h_stdin = h;
            }
        }

        let stdout_ok = !is_bad_handle(settings.h_stdout);
        let stderr_ok = !is_bad_handle(settings.h_stderr);
        let stdin_ok = !is_bad_handle(settings.h_stdin);

        if stdout_ok && stderr_ok && stdin_ok {
            return true;
        }

        if attempt < retries - 1 {
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
    }

    false
}

fn connect_to_pipe(name: &str, access: u32) -> Result<HANDLE, windows::core::Error> {
    let wide: Vec<u16> = std::os::windows::ffi::OsStr::new(name)
        .encode_wide()
        .chain(Some(0))
        .collect();

    unsafe {
        let wait_result = WaitNamedPipeW(PCWSTR(wide.as_ptr()), 0);
        if wait_result.as_bool() {
            let handle = CreateFileW(
                PCWSTR(wide.as_ptr()),
                access,
                0,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )?;
            Ok(handle)
        } else {
            Err(windows::core::Error::from_win32())
        }
    }
}

fn is_bad_handle(h: HANDLE) -> bool {
    h.is_invalid() || h == HANDLE(0)
}

fn build_secure_pipe_security_descriptor() -> Result<Vec<u8>, String> {
    use windows::Win32::Security::*;
    use std::mem;
    unsafe {
        let acl_size = mem::size_of::<ACL>() + mem::size_of::<ACCESS_ALLOWED_ACE>() * 2 + 64;
        let sd_alloc_len = SECURITY_DESCRIPTOR_MIN_LENGTH as usize;
        let total_size = sd_alloc_len + acl_size;

        let mut buf = vec![0u8; total_size];
        let sd_ptr = buf.as_mut_ptr() as *mut SECURITY_DESCRIPTOR;

        let mut sid_system = SID::default();
        let mut sid_auth_users = SID::default();
        let mut sid_size_system = mem::size_of::<SID>() as u32;
        let mut sid_size_auth = mem::size_of::<SID>() as u32;

        if CreateWellKnownSid(
            WinLocalSystemSid,
            None,
            Some(&mut sid_system as *mut SID as *mut std::ffi::c_void),
            &mut sid_size_system,
        )
        .is_err()
        {
            return Err("CreateWellKnownSid(SYSTEM) failed".into());
        }
        if CreateWellKnownSid(
            WinAuthenticatedUserSid,
            None,
            Some(&mut sid_auth_users as *mut SID as *mut std::ffi::c_void),
            &mut sid_size_auth,
        )
        .is_err()
        {
            return Err("CreateWellKnownSid(AuthenticatedUsers) failed".into());
        }

        if InitializeSecurityDescriptor(sd_ptr, SECURITY_DESCRIPTOR_REVISION).is_err() {
            return Err("InitializeSecurityDescriptor failed".into());
        }

        let acl_ptr = buf[sd_alloc_len..].as_mut_ptr() as *mut ACL;
        if InitializeAcl(acl_ptr, acl_size as u32, ACL_REVISION).is_err() {
            return Err("InitializeAcl failed".into());
        }

        if AddAccessAllowedAce(
            acl_ptr,
            ACL_REVISION,
            GENERIC_ALL,
            &sid_system as *const SID as *mut SID,
        )
        .is_err()
        {
            return Err("AddAccessAllowedAce(SYSTEM) failed".into());
        }

        if AddAccessAllowedAce(
            acl_ptr,
            ACL_REVISION,
            GENERIC_ALL,
            &sid_auth_users as *const SID as *mut SID,
        )
        .is_err()
        {
            return Err("AddAccessAllowedAce(AuthenticatedUsers) failed".into());
        }

        if SetSecurityDescriptorDacl(sd_ptr, true, acl_ptr, false).is_err() {
            return Err("SetSecurityDescriptorDacl failed".into());
        }

        Ok(buf)
    }
}

fn pipe_read_loop(
    read_handle: HANDLE,
    stop: Arc<AtomicBool>,
    write_fn: impl Fn(&[u8]) + Send + 'static,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut buffer = [0u8; PIPE_BUFFER_SIZE];
        loop {
            if stop.load(Ordering::Relaxed) {
                break;
            }

            let mut read = 0u32;
            let result = unsafe {
                ReadFile(read_handle, Some(&mut buffer), Some(&mut read), None)
            };

            if result.is_err() || read == 0 {
                break;
            }

            write_fn(&buffer[..read as usize]);
        }
    })
}

pub fn listen_remote_stdout(
    handle: HANDLE,
    stop: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    let h_output = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
    pipe_read_loop(handle, stop, move |data| {
        unsafe {
            let mut written = 0u32;
            WriteConsoleA(h_output, data, Some(&mut written), None).ok();
        }
    })
}

pub fn listen_remote_stderr(
    handle: HANDLE,
    stop: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    pipe_read_loop(handle, stop, |data| {
        let s = String::from_utf8_lossy(data);
        eprint!("{}", s);
    })
}

pub fn listen_remote_stdin(
    handle: HANDLE,
    stop: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let h_input = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
        let mut buffer = [0u8; PIPE_BUFFER_SIZE];
        loop {
            if stop.load(Ordering::Relaxed) {
                break;
            }

            let mut read = 0u32;
            let result = unsafe {
                ReadFile(h_input, Some(&mut buffer), Some(&mut read), None)
            };

            if result.is_err() || read == 0 {
                break;
            }

            let mut written = 0u32;
            unsafe {
                WriteFile(handle, Some(&buffer[..read as usize]), Some(&mut written), None).ok();
            }
        }
    })
}

pub fn update_settings_handles(
    settings: &mut RemoteSettings,
    pipes: &IoPipes,
) {
    settings.h_stdout = pipes.stdout_pipe;
    settings.h_stderr = pipes.stderr_pipe;
    settings.h_stdin = pipes.stdin_pipe;
}
