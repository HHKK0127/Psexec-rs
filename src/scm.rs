use windows::core::{Error, PCWSTR};
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Services::*;

use crate::settings::{get_service_name, RemoteSettings};

pub struct ScmConnection {
    handle: SC_HANDLE,
}

unsafe impl Send for ScmConnection {}
unsafe impl Sync for ScmConnection {}

impl ScmConnection {
    pub fn open(server: Option<&str>) -> Result<Self, Error> {
        let wide = server.map(|s| {
            std::os::windows::ffi::OsStr::new(s)
                .encode_wide()
                .chain(Some(0))
                .collect::<Vec<u16>>()
        });

        let handle = unsafe {
            OpenSCManagerW(
                wide.as_ref()
                    .map(|w| PCWSTR(w.as_ptr()))
                    .unwrap_or(PCWSTR::null()),
                None,
                SC_MANAGER_CONNECT | SC_MANAGER_CREATE_SERVICE,
            )?
        };

        Ok(Self { handle })
    }

    pub fn install_service(
        &self,
        name: &str,
        display_name: &str,
        binary_path: &str,
    ) -> Result<SC_HANDLE, Error> {
        let name_wide: Vec<u16> =
            std::os::windows::ffi::OsStr::new(name)
                .encode_wide()
                .chain(Some(0))
                .collect();
        let display_wide: Vec<u16> =
            std::os::windows::ffi::OsStr::new(display_name)
                .encode_wide()
                .chain(Some(0))
                .collect();
        let path_wide: Vec<u16> =
            std::os::windows::ffi::OsStr::new(binary_path)
                .encode_wide()
                .chain(Some(0))
                .collect();

        unsafe {
            let service = CreateServiceW(
                self.handle,
                PCWSTR(name_wide.as_ptr()),
                PCWSTR(display_wide.as_ptr()),
                SERVICE_START | SERVICE_STOP | SERVICE_QUERY_STATUS | DELETE,
                SERVICE_WIN32_OWN_PROCESS,
                SERVICE_DEMAND_START,
                SERVICE_ERROR_NORMAL,
                PCWSTR(path_wide.as_ptr()),
                None,
                None,
                None,
                None,
                None,
            )?;
            Ok(service)
        }
    }

    pub fn open_service(&self, name: &str) -> Result<SC_HANDLE, Error> {
        let name_wide: Vec<u16> =
            std::os::windows::ffi::OsStr::new(name)
                .encode_wide()
                .chain(Some(0))
                .collect();

        unsafe {
            let service = OpenServiceW(
                self.handle,
                PCWSTR(name_wide.as_ptr()),
                DELETE | SERVICE_QUERY_STATUS | SERVICE_STOP,
            )?;
            Ok(service)
        }
    }

    pub fn delete_service(service: SC_HANDLE) -> Result<(), Error> {
        unsafe {
            if DeleteService(service).as_bool() {
                Ok(())
            } else {
                Err(Error::from_win32())
            }
        }
    }

    pub fn close(&mut self) {
        if self.handle.is_invalid() {
            return;
        }
        unsafe {
            let _ = CloseServiceHandle(self.handle);
            self.handle = SC_HANDLE::default();
        }
    }
}

impl Drop for ScmConnection {
    fn drop(&mut self) {
        self.close();
    }
}

pub struct RemoteService {
    scm: ScmConnection,
    service_handle: Option<SC_HANDLE>,
}

impl RemoteService {
    pub fn install_and_start(
        server: Option<&str>,
        settings: &RemoteSettings,
        imp_pipe_name: &str,
    ) -> Result<Self, String> {
        let scm = ScmConnection::open(server).map_err(|e| {
            format!("Failed to open SCM: {}", e)
        })?;

        let service_name = get_service_name(settings);
        let binary_path = format!(
            "{}\\{}.exe -service -pipename {}",
            settings.target_share_path, service_name, imp_pipe_name
        );

        let service = match scm.install_service(
            &service_name,
            &service_name,
            &binary_path,
        ) {
            Ok(s) => s,
            Err(e) => {
                return Err(format!("Failed to install service: {}", e));
            }
        };

        unsafe {
            if StartServiceW(service, &[]).is_err() {
                let gle = Error::from_win32();
                if gle.code() != windows::Win32::Foundation::ERROR_SERVICE_ALREADY_RUNNING.0 {
                    let _ = CloseServiceHandle(service);
                    return Err(format!("Failed to start service: {}", gle));
                }
            }
        }

        Ok(Self {
            scm,
            service_handle: Some(service),
        })
    }

    pub fn stop_and_delete(&mut self, server: Option<&str>) {
        if let Some(service) = self.service_handle.take() {
            unsafe {
                let mut status = SERVICE_STATUS::default();
                let _ = ControlService(service, SERVICE_CONTROL_STOP, &mut status);

                for _ in 0..300 {
                    let mut ssp = SERVICE_STATUS_PROCESS::default();
                    let mut needed = 0u32;
                    if QueryServiceStatusEx(
                        service,
                        SC_STATUS_PROCESS_INFO,
                        Some(&mut ssp as *mut _ as *mut std::ffi::c_void),
                        std::mem::size_of::<SERVICE_STATUS_PROCESS>() as u32,
                        &mut needed,
                    )
                    .is_ok()
                    {
                        if ssp.dwCurrentState == SERVICE_STOPPED {
                            break;
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }

                let _ = DeleteService(service);
                let _ = CloseServiceHandle(service);
            }
        }
    }

    pub fn cleanup_orphaned(server: Option<&str>) {
        let my_pid = std::process::id();
        let scm = match ScmConnection::open(server) {
            Ok(s) => s,
            Err(_) => return,
        };

        unsafe {
            let mut buffer = [0u8; 63 * 1024];
            let mut needed = 0u32;
            let mut count = 0u32;
            let mut resume = 0u32;

            if EnumServicesStatusExW(
                scm.handle,
                SC_ENUM_PROCESS_INFO,
                SERVICE_WIN32_OWN_PROCESS,
                SERVICE_STATE_ALL,
                Some(&mut buffer as *mut _ as *mut std::ffi::c_void),
                buffer.len() as u32,
                &mut needed,
                &mut count,
                &mut resume,
                None,
            )
            .is_ok()
            {
                let entries = &buffer[..std::mem::size_of::<ENUM_SERVICE_STATUS_PROCESSW>() * count as usize];
                let ptr = entries.as_ptr() as *const ENUM_SERVICE_STATUS_PROCESSW;
                for i in 0..count {
                    let entry = &*ptr.add(i as usize);
                    if entry.ServiceStatusProcess.dwProcessId == my_pid {
                        let name_len = (0..)
                            .position(|i| *entry.lpServiceName.add(i) == 0)
                            .unwrap_or(0);
                        if let Ok(svc) = scm.open_service(
                            &String::from_utf16_lossy(
                                std::slice::from_raw_parts(
                                    entry.lpServiceName as *const u16,
                                    name_len,
                                ),
                            ),
                        ) {
                            let _ = DeleteService(svc);
                            let _ = CloseServiceHandle(svc);
                        }
                        break;
                    }
                }
            }
        }
    }
}

impl Drop for RemoteService {
    fn drop(&mut self) {
        self.stop_and_delete(None);
    }
}
