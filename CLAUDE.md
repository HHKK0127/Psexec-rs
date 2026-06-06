# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**PAExec-rs** is a Rust port of PsExec — a Windows remote command execution tool. It supports three operational modes: GUI (PE file analyzer), CLI (local or remote execution), and service (Windows service for handling remote requests).

## Build & Run Commands

```bash
# Build for debug
cargo build

# Build for release
cargo build --release

# Run GUI (PE file analyzer)
cargo run --release

# Run with CLI (requires arguments, see CLI usage)
cargo run --release -- <args>

# Run as service
cargo run --release -- -service

# Note: Current environment is network-restricted; offline builds may fail.
# A prebuilt binary exists at /deps/psexec_rs.exe (2.6 MB)
```

**Cargo features:** None currently defined. Windows 0.52 features are pinned in Cargo.toml to minimize build time.

## Project Structure

The application has three execution paths, controlled by command-line arguments:

### **Mode 1: GUI Mode** (default, no args or `--gui` flag)
- **Entry**: `main.rs:26` → `run_gui()`
- **Key files**: `ui.rs`, `analyzer.rs`, `winapi_utils.rs`
- **Purpose**: Interactive PE file analysis with tabs for Overview, PE Info, Imports, Strings, Signature
- **Architecture**: egui/eframe immediate-mode GUI; single-threaded, synchronous analysis
- **Known issues**: UI thread blocks during file analysis; no file size limit

### **Mode 2: CLI Mode** (args provided, e.g., `\\computername command args`)
- **Entry**: `main.rs:39` → `run_cli()` → `execute_remote()`
- **Key files**: `cli.rs`, `settings.rs`, `process.rs`, `remote.rs`, `scm.rs`
- **Purpose**: Execute commands locally or remotely via network shares and Windows service management
- **Flow**:
  1. Parse command-line arguments (computer list, settings, app to run)
  2. If no computers: start process locally (`process.rs`)
  3. If computers specified: for each, connect via admin share, copy executable, install service, run command
- **Key modules**:
  - `cli.rs` — argument parsing (PsExec-like syntax: `\\computer command`)
  - `settings.rs` — configuration structs (`RemoteSettings`, `RemoteFile`, etc.)
  - `process.rs` — local process creation via `CreateProcessW`
  - `remote.rs` — SMB/admin share management
  - `scm.rs` — Windows Service Control Manager (install, start, stop, delete services)
  - `proto.rs` — message protocol for pipe communication
  - `pipes.rs` — named pipe handling for service communication

### **Mode 3: Service Mode** (`-service` flag)
- **Entry**: `main.rs:123` → `run_service()`
- **Key files**: `pipes.rs`, `scm.rs`, `proto.rs`, `settings.rs`, `process.rs`
- **Purpose**: Runs as a Windows service to receive and execute commands from remote clients
- **Flow**:
  1. Create named pipe `\\.\pipe\PAExec-rs`
  2. Loop: accept connections, spawn thread per client
  3. Each thread: read message from pipe, deserialize settings, start process, return exit code
  4. Messages defined in `proto.rs` (MSGID_SETTINGS, MSGID_START_APP, etc.)

## Module Responsibilities

| Module | Lines | Purpose |
|--------|-------|---------|
| `main.rs` | 346 | Entry point; dispatches to GUI/CLI/service modes; wraps Win32 API calls |
| `analyzer.rs` | 220 | PE file parsing; hash computation; string extraction; version info querying |
| `ui.rs` | 211 | egui app state and rendering; five-tab UI for analysis results |
| `cli.rs` | ~150 | Command-line argument parsing (PsExec-compatible syntax) |
| `settings.rs` | ~150 | Configuration structs for local and remote execution |
| `process.rs` | ~100 | `CreateProcessW` wrapper; process handle/token management |
| `remote.rs` | ~100 | SMB/UNC path handling; admin share connection |
| `scm.rs` | ~150 | Windows Service Control Manager; install/start/stop/delete services |
| `pipes.rs` | ~100 | Named pipe creation and message handling |
| `proto.rs` | ~100 | Binary message serialization/deserialization |
| `winapi_utils.rs` | 135 | Win32 API helpers: version info, Authenticode signature verification |

## Critical Known Issues

### 1. **Buffer Over-Read in `winapi_utils.rs:76`** (UB — MUST FIX)
```rust
let slice = std::slice::from_raw_parts(ptr as *const u16, len as usize);
```
`VerQueryValueW` returns length in **bytes**, not elements. For "1.0.0.0\0" (16 bytes), this reads 32 bytes = 16-byte OOB read.

**Fix**: `len as usize / 2` to convert byte count to u16 count.

### 2. **UI Thread Blocking** (`analyzer.rs:62–98`)
File analysis (I/O, hashing, PE parsing, Win32 calls) runs synchronously on the UI thread. Files >1 second freeze the GUI completely.

**Fix**: Move to worker thread via `std::thread::spawn` + `std::sync::mpsc` channel.

### 3. **No File Size Limit** (`analyzer.rs:63`)
`fs::read(path)` loads entire file into memory with no upper bound. Multi-GB files cause OOM.

**Fix**: Add upper limit (suggested: 200 MB), validate before reading.

## Architecture Notes

### PE Analysis Pipeline (GUI Mode)
1. User opens file via native file dialog (`rfd` crate)
2. `analyzer::analyze_file()` (blocking):
   - Read file into memory
   - Compute SHA-256
   - Parse PE headers with `goblin::pe::PE`
   - Extract imports, sections, entry point
   - Query version resource via `GetFileVersionInfoW`
   - Verify Authenticode signature via `WinVerifyTrust`
   - Scan for ASCII/Unicode strings (keyword-filtered)
3. Results rendered in `AnalyzerApp` tabs

### Remote Execution Pipeline (CLI Mode, Multiple Computers)
1. Parse `\\computer1,computer2 app args`
2. For each computer:
   - `remote::connect_admin()` — authenticate to `\\computer\admin$` SMB share
   - `remote::copy_executable_to_remote()` — copy binary and any input files
   - `scm::install_and_start()` — create service, start it (triggers service mode)
   - Service spawns the requested process
   - Client waits for exit code via named pipe
   - `scm::stop_and_delete()` — remove service

### Named Pipe Protocol (Service ↔ Client)
Binary messages defined in `proto.rs`:
- `MSGID_SETTINGS` — client sends execution settings + file list
- `MSGID_RESP_SEND_FILES` — service requests client to send files
- `MSGID_SENT_FILES` — client confirms files received
- `MSGID_START_APP` — client signals to execute the app
- `MSGID_OK` / `MSGID_FAILED` — responses with exit code

## Testing

**Current status**: No unit tests, no integration tests, no CI.

**Recommended coverage**:
- `analyzer.rs`: Test PE parsing on known-good binaries; verify SHA-256; test string extraction; edge cases (non-PE files, corrupt headers, empty files)
- `winapi_utils.rs`: Test version info extraction and signature verification on signed/unsigned files
- `cli.rs`: Test argument parsing (computer lists, flags, app names)
- `process.rs`, `remote.rs`, `scm.rs`: Manual testing on a Windows domain (requires test VM with multiple computers)

## Development Workflow

### Adding a feature to GUI analyzer
1. Add analysis logic to `analyzer.rs` (add struct field to `AnalysisResult`)
2. Update `ui.rs` to render the new data in appropriate tab
3. Test via `cargo run --release`

### Adding a new CLI flag
1. Add field to `RemoteSettings` in `settings.rs`
2. Parse in `cli.rs:parse_command_line()`
3. Apply in `process.rs` or `scm.rs` (depending on scope)
4. Update usage text in `cli.rs:print_usage()`

### Fixing remote execution issues
1. Enable `RUST_LOG=debug cargo run --release -- <args>` to see debug output
2. Check `pipes.rs` for message flow (client → service)
3. Verify `proto.rs` serialization/deserialization
4. Test on actual remote computer or local VM

## Dependencies & Build Notes

Key dependencies (pinned versions in Cargo.toml):
- **egui 0.27** — UI framework (older version for stability)
- **windows 0.52** — Safe Win32 API bindings (version pinned to avoid breaking changes)
- **goblin 0.8** — Pure-Rust PE parser
- **rfd 0.14** — Native file dialogs
- **chrono 0.4** — Timestamp formatting
- **sha2, hex** — Hashing and encoding

**Network restriction**: Current environment is offline from crates.io. Prebuilt `/deps/psexec_rs.exe` is available for testing. If building from scratch is needed, configure a local registry mirror or vendored dependencies.

## Important Git Notes

From `HANDOFF.md`:
- Only `README.md` is currently tracked in git (commit `e033122`)
- Source files are **untracked working tree files**
- Build artifacts (`/target/`, `/deps/`, `.exe`, `.pdb`, `.rlib`) must be added to `.gitignore` before committing

## Deployment

**Binary**: Single statically-linked .exe (~2.6 MB release build)
**Distribution**: No external DLL dependencies (all linked statically)
**Security**: Binary is unsigned — users may see SmartScreen warnings on first run
**Permissions**: Requires admin privileges on local machine; requires domain credentials for remote execution

---

**See also**: `HANDOFF.md` for comprehensive design decisions, API documentation, and open questions.
