# Handoff Document: Psexec-rs (PE File Analyzer)

## 1. Summary of Work Completed

A Windows-native GUI application for analyzing PE (Portable Executable) files, purpose-built for examining PsExec and related binaries. Built with Rust using the `egui`/`eframe` immediate-mode GUI framework.

### What it does:
- Opens any PE file via native file dialog
- Displays file metadata (size, timestamps, SHA-256 hash)
- Parses PE headers (machine type, subsystem, entry point, image base, sections)
- Lists imported DLLs and their imported functions
- Extracts and filters interesting ASCII/Unicode strings from the binary
- Reads Windows version info (FileVersion, CompanyName, etc.)
- Validates Authenticode digital signatures via `WinVerifyTrust`

### Files:

| File | Lines | Purpose |
|------|-------|---------|
| `src/main.rs` | 16 | Entry point, window setup |
| `src/analyzer.rs` | 220 | Core PE analysis logic |
| `src/ui.rs` | 211 | egui UI rendering |
| `src/winapi_utils.rs` | 135 | Windows API bindings |
| `Cargo.toml` | 21 | Dependencies & features |
| `README.md` | 1 | Placeholder |

---

## 2. Current State and Known Issues

### Build Status
- **Does NOT compile** in the current environment (network restricted from crates.io)
- Last known build: `deps/psexec_rs.exe` (2.6 MB) in the `deps/` directory
- All source files are **untracked** in git (only `README.md` is committed to HEAD)

### Critical Bugs

**2a. Buffer over-read in `winapi_utils.rs:76`**
```rust
let slice = std::slice::from_raw_parts(ptr as *const u16, len as usize);
```
`VerQueryValueW`'s `puLen` returns length **in bytes**, but `from_raw_parts` expects element count. For a string like `"1.0.0.0\0"` (16 bytes = 8 WCHARs), this reads `16 * 2 = 32` bytes — a 16-byte OOB read (UB).

**Fix**: `let slice = std::slice::from_raw_parts(ptr as *const u16, len as usize / 2);`

**2b. UI thread blocking** — `analyzer.rs:62-98`
`analyze_file()` runs all I/O, hashing, PE parsing, and Win32 API calls synchronously on the UI thread. Any file delays (~1+ seconds) freeze the GUI completely.

**2c. No file size limit** — `analyzer.rs:63`
`fs::read(path)` loads the entire file into memory with no upper bound. A multi-GB file will exhaust RAM.

### Other Issues
- **Build artifacts committed** — `deps/`, `deps.zip.*`, `.exe`, `.pdb`, `.rlib` are all tracked. Need `.gitignore`.
- **PE timestamp raw** — displayed as Unix integer (e.g. `1735689600`) instead of a human date.
- **Machine/Subsystem raw hex** — shown as `0x8664`, `0x0002` without human-readable names.
- **Shared filter bar** — the same mutable `filter` string is shared between Imports and Strings tabs; switching tabs retains the filter.
- **SignatureInfo dead fields** — `.signer`, `.serial`, `.thumbprint`, `.valid_from`, `.valid_to` are declared but never populated.
- **No `.gitignore`** — entire `target/`, `deps/`, and build artifacts are eligible for tracking.

---

## 3. Design Decisions and Rationale

| Decision | Rationale |
|----------|-----------|
| **egui/eframe GUI** | Immediate-mode GUI avoids complex widget tree; single binary output; native Windows look via `winit`. Version `0.27` chosen for API stability. |
| **goblin for PE parsing** | Pure-Rust PE parser, no `unsafe`, actively maintained. Sufficient for header/import/section extraction. |
| **`windows` crate for Win32 APIs** | Official Microsoft Rust projection; type-safe bindings; avoids `unsafe` FFI boilerplate. Version `0.52` pinned. Features minimized to reduce build time. |
| **`rfd` for file dialogs** | Native OS file dialog; no extra GUI chrome. |
| **Monospace text display** | All analysis output shown in `ui.monospace()` for aligned columnar display. |
| **String keyword filtering** | Pre-filter to show only "interesting" strings (psexec, sysinternals, token, pipe, etc.) — reduces noise for the target use case. |
| **Synchronous API calls** | Initial prototype simplicity. Analysis is fast for typical files (< 10 MB). Async not yet implemented. |

---

## 4. API and Interface Documentation

### `src/analyzer.rs`

#### `pub fn analyze_file(path: &Path) -> Result<AnalysisResult, String>`
Reads a PE file and returns a comprehensive analysis result. Errors are returned as `String` for direct UI display.

**Returns `AnalysisResult`:**
| Field | Type | Description |
|-------|------|-------------|
| `file_path` | `String` | Full path to the analyzed file |
| `file_name` | `String` | File name only |
| `file_size` | `u64` | Size in bytes |
| `created` | `String` | Creation timestamp (ISO format) |
| `modified` | `String` | Modification timestamp (ISO format) |
| `sha256` | `String` | SHA-256 hex digest |
| `pe_info` | `PeInfo` | PE header information |
| `version_info` | `VersionInfo` | Windows version resource data |
| `signature` | `SignatureInfo` | Authenticode signature status |
| `imports` | `Vec<ImportDll>` | List of imported DLLs |
| `strings_ascii` | `Vec<String>` | Filtered ASCII strings |
| `strings_unicode` | `Vec<String>` | Filtered UTF-16 strings |

#### `fn compute_sha256(data: &[u8]) -> String`
SHA-256 hash via `sha2` crate, output as lowercase hex.

#### `fn extract_pe_info(pe: &PE) -> PeInfo`
Parses machine type, optional header, sections from `goblin::pe::PE`.

#### `fn extract_imports(pe: &PE) -> Vec<ImportDll>`
Extracts DLL names and imported function symbols (by name or ordinal).

#### `fn extract_strings(data: &[u8]) -> (Vec<String>, Vec<String>)`
Scans binary for printable ASCII strings (>=4 chars, bytes 0x20-0x7E) and naive "Unicode" strings (`char + '\0'` pattern). Filters against a keyword allowlist. Minimum 6 chars after filtering.

### `src/winapi_utils.rs`

#### `pub fn get_version_info(path: &Path) -> VersionInfo`
Reads `VS_VERSIONINFO` resource via `GetFileVersionInfoSizeW` / `GetFileVersionInfoW` / `VerQueryValueW`. Queries `\StringFileInfo\{lang}\*` keys. Returns empty fields on failure.

#### `pub fn verify_signature(path: &Path) -> SignatureInfo`
Calls `WinVerifyTrust` with `WINTRUST_ACTION_GENERIC_VERIFY_V2`. Returns only "Valid" or HRESULT error code. Does NOT extract certificate chain details (signer, serial, etc.).

#### `unsafe fn query_value(buffer: &[u8], key: &str) -> String`
Internal helper: runs `VerQueryValueW` for a given key and returns the string value.

### `src/ui.rs`

`AnalyzerApp` struct manages app state. Implements `eframe::App` trait. Five tabs: Overview, PE Info, Imports, Strings, Signature. Imports and Strings tabs include a text filter field.

### `src/main.rs`

Minimal entry point. Creates 960x780 window titled "PE File Analyzer" via `eframe::run_native`.

---

## 5. Configuration and Environment Notes

### Requirements
- **OS**: Windows x86-64 only (uses `windows` crate + Win32 APIs)
- **Rust**: edition 2021, toolchain stable
- **Build**: `cargo build --release`

### Dependencies (Cargo.toml)
```
eframe = "0.27"       # GUI framework (includes egui, winit, glow/wgpu)
egui = "0.27"         # Immediate-mode GUI
goblin = "0.8"        # PE parsing
sha2 = "0.10"         # SHA-256 hashing
hex = "0.4"           # Hex encoding
rfd = "0.14"          # Native file dialogs
chrono = "0.4"        # Timestamp formatting
windows = "0.52"      # Win32 API bindings
  features: Win32_Foundation, Win32_Security_WinTrust, Win32_Storage_FileSystem
```

### Build Artifacts (do not commit)
- `/target/` — cargo build output
- `/deps/` — prebuilt dependencies (2.6 MB .exe, .rlib, .pdb, .dll)
- `deps.zip.001/002/003` — 86 MB of zipped deps

### Environment variables
None required. Network access needed for first `cargo build` to download crate dependencies.

---

## 6. Testing Status

**No tests exist.** The project has zero unit tests, integration tests, or CI configuration.

### Recommended test coverage:
- **Unit tests for `analyzer.rs`**: Test PE parsing against known-good PE files, SHA-256 correctness, string extraction, import parsing. Test edge cases: non-PE files, corrupt headers, empty files, very large files.
- **Test for `winapi_utils.rs`**: Test version info extraction against files with/without version resources. Test signature verification against signed/unsigned/modified files.
- **UI smoke test**: Manual test — open file, verify all 5 tabs render correctly, test filter functionality.

---

## 7. Deployment Considerations

### Distribution
- Single `psexec_rs.exe` binary (~2.6 MB release build with static CRT)
- No external DLL dependencies (Rust + Windows API statically linked)
- Windows Defender / SmartScreen may flag the unsigned binary

### Security
- The binary itself is **not Authenticode-signed** — users may see SmartScreen warnings
- Analysis output includes the full file path — could leak sensitive path information if logs are shared
- No sandboxing for the analyzed file (the app reads arbitrary PE files from disk)

### Performance
- Memory usage ~= analyzed file size + ~30 MB overhead
- Startup time: ~1 second (egui/winit initialization)
- No background/async processing; analysis is single-threaded and synchronous

---

## 8. Open Questions and Next Steps

### Priority fixes (in order):
1. **Fix OOB read** in `winapi_utils.rs:76` (UB bug)
2. **Add `.gitignore`** — remove build artifacts from tracking
3. **Add file size limit** — prevent OOM on large files
4. **Add `.cargo/config.toml`** with registry mirror if building behind corporate proxy
5. **Move analysis off UI thread** — use `std::thread::spawn` + channels
6. **Decode PE timestamp** to human-readable date
7. **Add human-readable machine/subsystem names**

### Enhancements:
- Implement certificate chain traversal via `CryptQueryObject` / `CertGetCertificateChain` to populate `signer`, `serial`, `thumbprint`, etc.
- Add export table parsing
- Add resource directory parsing (icons, manifests, etc.)
- Add progress indicator during file analysis
- Add drag-and-drop file loading
- Add comparison mode (analyze two files side-by-side)
- Generate report as JSON or HTML

### Open questions:
- Should analysis be async (tokio) or just a threaded `std::sync::mpsc` channel?
- What is the maximum file size to allow? (Suggested: 200 MB)
- Should the application support analyzing files from network paths (UNC)?
- Is there a need for batch analysis of multiple files?

---

## 9. Recommended Contacts and Resources

### Project contact
- **Author**: Hiroki Kogarumai (`HHKK0127`)
- **Email**: Hiroki.Kogarumai@protonmail.com

### Key documentation
- [egui documentation](https://docs.rs/egui/0.27/)
- [eframe documentation](https://docs.rs/eframe/0.27/)
- [goblin PE parsing](https://docs.rs/goblin/0.8/goblin/pe/index.html)
- [windows crate docs](https://docs.rs/windows/0.52.0/windows/)
- [WinVerifyTrust API](https://learn.microsoft.com/en-us/windows/win32/api/wintrust/nf-wintrust-winverifytrust)
- [Version information APIs](https://learn.microsoft.com/en-us/windows/win32/menurc/version-information)

### Git notes
- Only `README.md` is tracked in HEAD (commit `e033122`)
- Source files are **untracked working tree files**
- Commit `687f1c0` is a GitHub Desktop merge commit
- Commit `9e39504` is a stash containing old build artifacts
