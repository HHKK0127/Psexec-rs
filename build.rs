fn main() {
    #[cfg(target_os = "windows")]
    {
        // Windows リソースにアイコンを設定（icon.icoが存在する場合）
        let icon_path = "assets/icons/icon.ico";
        if std::path::Path::new(icon_path).exists() {
            winresource::WindowsResource::new()
                .set_icon(icon_path)
                .set("ProductName", "PAExec-rs")
                .set("FileDescription", "Windows Remote Command Execution Tool")
                .set("ProductVersion", "1.0.0.0")
                .set("FileVersion", "1.0.0.0")
                .set("InternalName", "psexec-rs")
                .set("OriginalFilename", "psexec_rs.exe")
                .compile()
                .unwrap();
        } else {
            // Skip icon if not available
            println!("cargo:warning=Icon file not found at {}, skipping icon embedding", icon_path);
        }
    }
}
