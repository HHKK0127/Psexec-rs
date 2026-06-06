use crate::proto::RemMsg;
use std::path::Path;
use windows::Win32::Foundation::HANDLE;

#[derive(Clone, Default, Debug)]
pub struct FileInfo {
    pub filename_only: String,
    pub full_file_path: String,
    pub file_version_ms: u32,
    pub file_version_ls: u32,
    pub file_last_write: u64,
    pub copy_file: bool,
}

#[derive(Clone, Debug)]
pub struct RemoteSettings {
    pub allowed_processors: Vec<u16>,
    pub copy_files: bool,
    pub force_copy: bool,
    pub copy_if_newer_or_higher_ver: bool,
    pub dont_wait_for_terminate: bool,
    pub dont_load_profile: bool,
    pub session_to_interact_with: i32,
    pub interactive: bool,
    pub run_elevated: bool,
    pub run_limited: bool,
    pub password: String,
    pub user: String,
    pub use_system_account: bool,
    pub working_dir: String,
    pub show_ui_on_winlogon: bool,
    pub priority: u32,
    pub app: String,
    pub app_args: String,
    pub disable_file_redirection: bool,
    pub remote_log_path: String,
    pub no_delete: bool,
    pub src_dir: String,
    pub dest_dir: String,
    pub src_file_infos: Vec<FileInfo>,
    pub dest_file_infos: Vec<FileInfo>,
    pub timeout_seconds: u32,

    pub remote_comp_connect_timeout_sec: u32,
    pub computer_list: Vec<String>,
    pub target_share: String,
    pub target_share_path: String,
    pub no_name: bool,
    pub service_name: String,

    pub h_stdout: HANDLE,
    pub h_stderr: HANDLE,
    pub h_stdin: HANDLE,
}

impl Default for RemoteSettings {
    fn default() -> Self {
        Self {
            allowed_processors: Vec::new(),
            copy_files: false,
            force_copy: false,
            copy_if_newer_or_higher_ver: false,
            dont_wait_for_terminate: false,
            dont_load_profile: false,
            session_to_interact_with: -1,
            interactive: false,
            run_elevated: false,
            run_limited: false,
            password: String::new(),
            user: String::new(),
            use_system_account: false,
            working_dir: String::new(),
            show_ui_on_winlogon: false,
            priority: 0x20, // NORMAL_PRIORITY_CLASS
            app: String::new(),
            app_args: String::new(),
            disable_file_redirection: false,
            remote_log_path: String::new(),
            no_delete: false,
            src_dir: String::new(),
            dest_dir: String::new(),
            src_file_infos: Vec::new(),
            dest_file_infos: Vec::new(),
            timeout_seconds: 0,
            remote_comp_connect_timeout_sec: 0,
            computer_list: Vec::new(),
            target_share: "ADMIN$".into(),
            target_share_path: "%SYSTEMROOT%".into(),
            no_name: false,
            service_name: String::new(),

            h_stdout: HANDLE(0),
            h_stderr: HANDLE(0),
            h_stdin: HANDLE(0),
        }
    }
}

impl RemoteSettings {
    pub fn serialize(&self, msg: &mut RemMsg) {
        msg.serialize_u32(1); // version
        msg.serialize_u32(self.allowed_processors.len() as u32);
        for &p in &self.allowed_processors {
            msg.serialize_u32(p as u32);
        }
        msg.serialize_bool(self.copy_files);
        msg.serialize_bool(self.force_copy);
        msg.serialize_bool(self.copy_if_newer_or_higher_ver);
        msg.serialize_bool(self.dont_wait_for_terminate);
        msg.serialize_bool(self.dont_load_profile);
        msg.serialize_u32(self.session_to_interact_with as u32);
        msg.serialize_bool(self.interactive);
        msg.serialize_bool(self.run_elevated);
        msg.serialize_bool(self.run_limited);
        msg.serialize_string(&self.password);
        msg.serialize_string(&self.user);
        msg.serialize_bool(self.use_system_account);
        msg.serialize_string(&self.working_dir);
        msg.serialize_bool(self.show_ui_on_winlogon);
        msg.serialize_u32(self.priority);
        msg.serialize_string(&self.app);
        msg.serialize_string(&self.app_args);
        msg.serialize_bool(self.disable_file_redirection);
        msg.serialize_bool(false); // bODS
        msg.serialize_string(&self.remote_log_path);
        msg.serialize_bool(self.no_delete);
        msg.serialize_string(&self.src_dir);
        msg.serialize_string(&self.dest_dir);
        msg.serialize_u32(self.src_file_infos.len() as u32);
        for fi in &self.src_file_infos {
            msg.serialize_string(&fi.filename_only);
            msg.serialize_u64(fi.file_last_write);
            msg.serialize_u32(fi.file_version_ls);
            msg.serialize_u32(fi.file_version_ms);
        }
        msg.serialize_u32(self.dest_file_infos.len() as u32);
        for fi in &self.dest_file_infos {
            msg.serialize_string(&fi.filename_only);
            msg.serialize_u64(fi.file_last_write);
            msg.serialize_u32(fi.file_version_ls);
            msg.serialize_u32(fi.file_version_ms);
        }
        msg.serialize_u32(self.timeout_seconds);
    }

    pub fn deserialize(msg: &mut RemMsg) -> Self {
        let mut s = Self::default();
        let _version = msg.deserialize_u32();
        let num = msg.deserialize_u32();
        for _ in 0..num {
            s.allowed_processors.push(msg.deserialize_u32() as u16);
        }
        s.copy_files = msg.deserialize_bool();
        s.force_copy = msg.deserialize_bool();
        s.copy_if_newer_or_higher_ver = msg.deserialize_bool();
        s.dont_wait_for_terminate = msg.deserialize_bool();
        s.dont_load_profile = msg.deserialize_bool();
        s.session_to_interact_with = msg.deserialize_u32() as i32;
        s.interactive = msg.deserialize_bool();
        s.run_elevated = msg.deserialize_bool();
        s.run_limited = msg.deserialize_bool();
        s.password = msg.deserialize_string();
        s.user = msg.deserialize_string();
        s.use_system_account = msg.deserialize_bool();
        s.working_dir = msg.deserialize_string();
        s.show_ui_on_winlogon = msg.deserialize_bool();
        s.priority = msg.deserialize_u32();
        s.app = msg.deserialize_string();
        s.app_args = msg.deserialize_string();
        s.disable_file_redirection = msg.deserialize_bool();
        let _bods = msg.deserialize_bool();
        s.remote_log_path = msg.deserialize_string();
        s.no_delete = msg.deserialize_bool();
        s.src_dir = msg.deserialize_string();
        s.dest_dir = msg.deserialize_string();
        let num = msg.deserialize_u32();
        for _ in 0..num {
            let mut fi = FileInfo::default();
            fi.filename_only = msg.deserialize_string();
            fi.file_last_write = msg.deserialize_u64();
            fi.file_version_ls = msg.deserialize_u32();
            fi.file_version_ms = msg.deserialize_u32();
            s.src_file_infos.push(fi);
        }
        let num = msg.deserialize_u32();
        for _ in 0..num {
            let mut fi = FileInfo::default();
            fi.filename_only = msg.deserialize_string();
            fi.file_last_write = msg.deserialize_u64();
            fi.file_version_ls = msg.deserialize_u32();
            fi.file_version_ms = msg.deserialize_u32();
            s.dest_file_infos.push(fi);
        }
        s.timeout_seconds = msg.deserialize_u32();
        s
    }

    pub fn resolve_file_paths(&mut self) -> bool {
        let mut all_found = true;
        let p_dir = if self.dest_dir.is_empty() {
            &mut self.src_dir
        } else {
            &mut self.dest_dir
        };
        let p_file_list = if self.dest_dir.is_empty() {
            &mut self.src_file_infos
        } else {
            &mut self.dest_file_infos
        };

        for fi in p_file_list.iter_mut() {
            let mut path = if p_dir.is_empty() {
                String::new()
            } else {
                let d = p_dir.trim_end_matches('\\');
                format!("{}\\{}", d, fi.filename_only)
            };

            if path.is_empty() {
                path = fi.filename_only.clone();
            }

            if Path::new(&path).exists() {
                fi.full_file_path = path;
            } else if let Some(found) = expand_to_full_path(&fi.filename_only) {
                fi.full_file_path = found;
            } else {
                all_found = false;
            }

            get_target_file_info(fi);
        }

        all_found
    }
}

fn expand_to_full_path(name: &str) -> Option<String> {
    let paths = std::env::var("PATH").unwrap_or_default();
    for p in std::env::split_paths(&paths) {
        let candidate = p.join(name);
        if candidate.exists() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }
    None
}

fn get_target_file_info(fi: &mut FileInfo) {
    if fi.full_file_path.is_empty() {
        return;
    }
    if let Ok(meta) = std::fs::metadata(&fi.full_file_path) {
        if let Ok(modified) = meta.modified() {
            let duration = modified
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            fi.file_last_write = duration.as_secs();
        }
    }
}

pub fn get_service_name(settings: &RemoteSettings) -> String {
    if settings.no_name {
        "PAExec".into()
    } else if !settings.service_name.is_empty() {
        settings.service_name.clone()
    } else {
        let hostname = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "UNKNOWN".into());
        format!("PAExec-{}-{}", std::process::id(), hostname)
    }
}
