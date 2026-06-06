use std::path::Path;
use std::process::Command;

use crate::settings::{get_service_name, RemoteSettings};

pub struct NetConnection {
    server: String,
    share: String,
}

impl NetConnection {
    pub fn connect_admin(
        server: &str,
        share: &str,
        user: &str,
        password: &str,
    ) -> Result<Self, String> {
        let remote = format!(r"\\{}\{}", server, share);
        Self::connect(&remote, user, password)
    }

    pub fn connect_ipc(
        server: &str,
        user: &str,
        password: &str,
    ) -> Result<Self, String> {
        let remote = format!(r"\\{}\IPC$", server);
        Self::connect(&remote, user, password)
    }

    fn connect(remote: &str, user: &str, password: &str) -> Result<Self, String> {
        if remote.starts_with(r"\\.\") {
            return Ok(Self {
                server: ".".into(),
                share: String::new(),
            });
        }

        let mut cmd = Command::new("net");
        cmd.arg("use");
        cmd.arg(remote);

        if !user.is_empty() {
            cmd.arg(&format!("/user:{}", user));
        }
        if !password.is_empty() {
            cmd.arg(password);
        }

        let output = cmd.output().map_err(|e| format!("net use failed: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("net use {} failed: {}", remote, stderr.trim()));
        }

        let parts: Vec<&str> = remote.trim_start_matches(r"\\").split('\\').collect();
        Ok(Self {
            server: parts.first().unwrap_or(&"").to_string(),
            share: parts.get(1).unwrap_or(&"").to_string(),
        })
    }

    pub fn disconnect(&self) {
        if self.server == "." {
            return;
        }
        let remote = format!(r"\\{}\{}", self.server, self.share);
        let _ = Command::new("net")
            .args(&["use", &remote, "/delete"])
            .output();
    }
}

pub fn copy_file(
    local_path: &str,
    dest_path: &str,
) -> Result<(), String> {
    let src = Path::new(local_path);
    let dst = Path::new(dest_path);

    if !src.exists() {
        return Err(format!("Source file not found: {}", local_path));
    }

    if src == dst {
        return Ok(());
    }

    std::fs::copy(src, dst)
        .map_err(|e| format!("Failed to copy {} -> {}: {}", local_path, dest_path, e))?;

    log::info!("Copied {} -> {}", local_path, dest_path);
    Ok(())
}

pub fn delete_file(path: &str) {
    let p = Path::new(path);
    if p.exists() {
        for _ in 0..70 {
            if std::fs::remove_file(p).is_ok() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}

pub fn copy_executable_to_remote(
    server: &str,
    settings: &RemoteSettings,
) -> Result<String, String> {
    let share = &settings.target_share;
    let remote_exe_name = format!("{}.exe", get_service_name(settings));

    let local_path = std::env::current_exe()
        .map_err(|e| format!("Cannot get current exe path: {}", e))?
        .to_string_lossy()
        .to_string();

    let dest = if server == "." {
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
        format!(r"{}\{}", windir, remote_exe_name)
    } else {
        format!(r"\\{}\{}\{}", server, share, remote_exe_name)
    };

    if local_path == dest {
        return Ok(dest);
    }

    copy_file(&local_path, &dest)?;

    Ok(dest)
}

pub fn send_file_to_remote(
    server: &str,
    share: &str,
    local_path: &str,
    remote_filename: &str,
) -> Result<String, String> {
    let dest = if server == "." {
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
        format!(r"{}\{}", windir, remote_filename)
    } else {
        format!(r"\\{}\{}\{}", server, share, remote_filename)
    };

    copy_file(local_path, &dest)?;
    Ok(dest)
}
