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
| `main.rs` | 346 | Entry point; dispatches to GUI/CLI/service modes; tokio runtime integration |
| `analyzer.rs` | 220 | PE file parsing; hash computation; string extraction; version info querying |
| `ui.rs` | 1000+ | egui app state and rendering; eight-tab UI with async task management; color-coded status messages |
| `cli.rs` | ~150 | Command-line argument parsing (PsExec-compatible syntax; ModernCli integration) |
| `cli_handlers.rs` | 650+ | Six async handler functions for exec/service/registry/script/transfer/shell commands |
| `cli_response.rs` | 135 | Structured response types (ServiceListResponse, RegistryListResponse, etc.) with ServiceState enum |
| `config.rs` | 350+ | ConfigLoader (ENV > INI > JSON > Default); AppConfig persistence; CacheEntry<T> generic |
| `settings.rs` | ~150 | Configuration structs for local and remote execution |
| `process.rs` | ~100 | `CreateProcessW` wrapper; process handle/token management |
| `remote.rs` | ~100 | SMB/UNC path handling; admin share connection |
| `scm.rs` | ~150 | Windows Service Control Manager; install/start/stop/delete services |
| `pipes.rs` | ~100 | Named pipe creation and message handling |
| `proto.rs` | ~100 | Binary message serialization/deserialization |
| `winapi_utils.rs` | 135 | Win32 API helpers: version info, Authenticode signature verification |
| `executor/batch.rs` | 200+ | Batch processing engine; Semaphore-based concurrency control (default 10 parallel) |
| `executor/logging.rs` | 150+ | File logging + TTL caching (default 24h); ResultCache manager |
| `executor/pool.rs` | 200+ | PooledConnection; ConnectionPool; FailoverManager with round-robin + health checks |
| `gui/batch_panel.rs` | 300+ | Batch execution UI; progress visualization; async event handling |
| `gui/log_viewer.rs` | 250+ | Log viewer panel; filtering; auto-append from execution results |
| `script/executor.rs` | 200+ | Async script execution (PowerShell, VBScript, Batch, JavaScript); tokio-based |

## Implementation Status

### ✅ Phase 1-11 Complete

**Phase 1-3**: Remote execution foundation (auth, execution methods, file transfer, output capture, service management, registry ops, script execution)  
**Phase 8**: Batch processing (Semaphore concurrency), Logging + TTL caching, Config management (ENV > INI > JSON), Script execution (async, 4 languages)  
**Phase 9-11**: GUI integration (BatchPanel, LogViewerPanel), Connection pooling with failover, Retry policies (exponential backoff)

**Test Coverage**: 20+ unit tests passing  
**Build Status**: Release binary (2.6 MB) - statically linked, no DLL dependencies  
**New Files**: 20 files, 3,000+ lines of code added  
**Async Pattern**: tokio::runtime + Arc/Mutex/RwLock for thread-safe concurrent execution

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

### Batch Processing Pipeline (Phase 8)
1. User submits batch via GUI `BatchPanel` → `execute_batch(hosts, commands, settings)`
2. `executor::batch::BatchExecutor` splits work:
   - `Semaphore::new(max_concurrent)` controls parallelism (default 10)
   - Spawn async task per (host, command) pair
   - Each task: authenticate → execute → log result
3. Results collected in channel, UI updates progress bar in real-time
4. Completion triggers auto-append to `LogViewerPanel`

### Configuration Loading (Phase 8)
Priority: Environment Variables > INI file > JSON config > Hardcoded defaults
- `ConfigLoader::new()` checks `$PSEXEC_RS_CONFIG` env var
- Falls back to `.psexec-rs.ini` (Windows INI format)
- Falls back to `.psexec-rs.json` (JSON format)
- Uses defaults if none found

### Connection Pooling & Failover (Phase 10-11)
- `ConnectionPool<T>` maintains cache of authenticated SMB/WMI connections
- `FailoverManager` implements round-robin + health checks
- `RetryPolicy` with exponential backoff (1s → 2s → 4s → max 60s)
- Stale connections evicted; auto-reconnect on failure

## Testing

**Current status**: No unit tests, no integration tests, no CI.

**Recommended coverage**:
- `analyzer.rs`: Test PE parsing on known-good binaries; verify SHA-256; test string extraction; edge cases (non-PE files, corrupt headers, empty files)
- `winapi_utils.rs`: Test version info extraction and signature verification on signed/unsigned files
- `cli.rs`: Test argument parsing (computer lists, flags, app names)
- `process.rs`, `remote.rs`, `scm.rs`: Manual testing on a Windows domain (requires test VM with multiple computers)

## Development Workflow (Phase 1-11)

### Adding a feature to GUI analyzer
1. Add analysis logic to `analyzer.rs` (add struct field to `AnalysisResult`)
2. Update `ui.rs` to render the new data in appropriate tab
3. Test via `cargo run --release`

### Adding a new Modern CLI command
1. Add variant to `Commands` enum in `cli.rs`
2. Create handler in `cli_handlers.rs` (async function)
3. Define response type in `cli_response.rs`
4. Parse in ModernCli derive macro
5. Wire in `main.rs:run_modern_cli()`
6. Test: `cargo test` and `cargo run --release -- <command>`

### Integrating GUI with CLI Handlers
1. Add state fields to `AnalyzerApp` in `ui.rs` (e.g., `service_list_rx`)
2. Create spawning function (e.g., `spawn_service_list_task()`)
3. In button click: call spawn function with host/filter parameters
4. In `update()`: check `try_recv()` for async results
5. Update UI state fields with received data
6. Display with color coding (Green=success, Red=error, Blue=info)

### Fixing remote execution issues
1. Enable `RUST_LOG=debug cargo run --release -- <args>` to see debug output
2. Check `pipes.rs` for message flow (client → service)
3. Verify `proto.rs` serialization/deserialization
4. Test on actual remote computer or local VM

### Configuration & Caching
1. Use `AppConfig::load()` on startup to restore user preferences
2. Call `config.save()` after configuration changes
3. Check `ResultCache::is_valid()` before using cached results
4. TTL is configurable via `cache.ttl_seconds`

### Adding Batch Processing Features (Phase 8)
1. Define new batch command in `cli.rs` or UI dialog in `batch_panel.rs`
2. Call `BatchExecutor::execute()` with host list, command list, and `max_concurrent`
3. Batch executor spawns async tasks via tokio; respects Semaphore limits
4. Log each result via `logging::log_execution()`; results cached for 24h
5. Update progress bar in UI via mpsc channel
6. Verify via `cargo test --release` and manual GUI test

### Adding Connection Pooling (Phase 10-11)
1. Initialize `ConnectionPool::new()` in `main.rs` or thread-local storage
2. Before remote execution: `pool.get_or_create(host, auth_method)?`
3. On connection failure: `failover_mgr.next_endpoint()` routes to alternate server
4. Stale connections auto-evicted; health checks run periodically
5. Retry policy configured via `RetryPolicy::exponential()` with custom params

## Dependencies & Build Notes

Key dependencies (pinned versions in Cargo.toml):
- **egui 0.27** — UI framework (older version for stability; immediate-mode)
- **windows 0.52** — Safe Win32 API bindings (version pinned to avoid breaking changes)
- **goblin 0.8** — Pure-Rust PE parser
- **rfd 0.14** — Native file dialogs
- **chrono 0.4** — Timestamp formatting
- **sha2, hex** — Hashing and encoding
- **tokio** — Async runtime for non-blocking I/O (batch, connection pooling, retry logic)
- **clap 4.4** — Modern CLI argument parsing (derive API)
- **serde_json** — Configuration persistence
- **dirs** — Cross-platform config/cache directory paths
- **tempfile** — Secure temporary file handling for batch transfers
- **tracing** — Structured logging for debug/audit trail

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
