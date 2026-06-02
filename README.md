# PAExec-rs

A Windows remote command execution tool written in Rust. A port of PsExec with three operational modes: GUI PE file analyzer, CLI remote execution, and Windows service agent.

![Rust](https://img.shields.io/badge/Rust-1.70+-red)
![Windows](https://img.shields.io/badge/Platform-Windows%20x86--64-blue)
![License](https://img.shields.io/badge/License-MIT-green)

## Features

### 🔍 Mode 1: GUI PE File Analyzer
- Interactive PE file analysis with native Windows file dialog
- Display metadata: size, timestamps, SHA-256 hash
- Parse PE headers: machine type, subsystem, entry point, sections
- Extract imports: DLL names and imported functions
- String extraction: filtered ASCII/Unicode strings from binary
- Version information: FileVersion, CompanyName, ProductName, etc.
- Authenticode signature verification via WinVerifyTrust
- Real-time filtering with search tabs

### 🖥️ Mode 2: CLI Remote Execution
- Execute commands on local or remote computers
- PsExec-like syntax: `\\computername command args`
- Parallel execution on multiple computers
- Automatic service installation/cleanup
- File transfer to remote targets
- Named pipe communication protocol

### 🔧 Mode 3: Windows Service Agent
- Runs as a Windows service to receive remote commands
- Named pipe listener at `\\.\pipe\PAExec-rs`
- Binary message serialization/deserialization
- Process execution and exit code reporting
- Automatic cleanup of orphaned services

---

## Installation

### Prebuilt Binary
A prebuilt release binary is available at `/deps/psexec_rs.exe` (2.6 MB).

### Build from Source

**Requirements:**
- Windows x86-64
- Rust 1.70+ (stable)
- Internet access to crates.io (first build only)

**Build commands:**

```bash
# Debug build
cargo build

# Release build (optimized, ~2.6 MB)
cargo build --release

# Output: target/release/psexec_rs.exe
```

---

## Usage

### GUI Mode (PE File Analyzer)

```bash
# Launch GUI (default, no arguments)
cargo run --release

# Or with explicit flag
cargo run --release -- --gui
```

**Features:**
- Five tabs: Overview, PE Info, Imports, Strings, Signature
- Click "Open File" to select a PE binary
- Filter imports and strings with search fields

### CLI Mode (Remote Execution)

```bash
# Execute locally
cargo run --release -- notepad.exe

# Execute on single remote computer
cargo run --release -- \\computername cmd /c "whoami"

# Execute on multiple computers (comma-separated)
cargo run --release -- \\comp1,comp2,comp3 cmd /c "ipconfig"
```

**PsExec-compatible syntax:**
```
psexec [options] \\computer,computer2 application [args]
```

**Options:**
- `-u user` — specify username
- `-p password` — specify password
- `-c` — copy executable to remote before execution
- `-f` — force copy (overwrite existing)
- `-d` — don't wait for process termination
- `-t timeout` — timeout in seconds

### Service Mode

```bash
# Install and run as service
cargo run --release -- -service
```

Service listens for connections on named pipe and executes commands received from remote clients.

---

## Architecture

### Directory Structure

```
src/
├── main.rs              # Entry point; dispatcher (GUI/CLI/Service modes)
├── analyzer.rs          # PE file parsing and analysis
├── ui.rs                # egui/eframe GUI rendering
├── cli.rs               # Command-line argument parsing
├── settings.rs          # Configuration structs
├── process.rs           # CreateProcessW wrapper
├── remote.rs            # SMB/UNC path handling
├── scm.rs               # Service Control Manager (install/start/stop)
├── pipes.rs             # Named pipe creation and messaging
├── proto.rs             # Binary message protocol
└── winapi_utils.rs      # Win32 API helpers (version info, signatures)
```

### Execution Flow

**GUI Mode:**
1. User opens file via native dialog
2. `analyzer::analyze_file()` synchronously parses PE
3. Results displayed in egui tabs

**CLI Mode (Remote):**
1. Parse command-line arguments
2. For each computer: connect to admin share → copy executable → install service → run command
3. Service (started remotely) executes process and returns exit code
4. Client receives result via named pipe

**Service Mode:**
1. Create named pipe listener
2. Accept client connections in loop
3. Each client: deserialize settings → execute process → return exit code

---

## Recent Changes

### Version 0.1.0 (2026-05-29)

**Security Fixes:**
- Fix buffer over-read (undefined behavior) in VerQueryValueW parsing
- Add 100MB file size limit to prevent DoS/OOM attacks

**UX Improvements:**
- Convert PE timestamp to human-readable UTC format (e.g., "2025-01-01 12:00:00 UTC")
- Map machine type to readable names (i386, AMD64, ARM64, etc.)
- Map subsystem to readable names (GUI, CUI, Native, etc.)

**Code Refactor:**
- Eliminate duplicate metadata file system calls

---

## Known Issues & Limitations

### Current Status
- ❌ No unit tests or integration tests
- ❌ No CI/CD pipeline
- ⚠️ Binary is unsigned (SmartScreen warnings on first run)
- ⚠️ Network-restricted build environment (offline builds may fail)

### UI Thread Blocking
File analysis (hashing, PE parsing, Win32 calls) blocks the GUI thread. Large files (>1 second) cause UI freezing. Planned: move analysis to worker thread.

### Architecture Notes
- Synchronous I/O only (no async/await)
- Single-threaded GUI (egui immediate-mode)
- No certificate chain extraction (signature verification only)

---

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| egui/eframe | 0.27 | GUI framework (immediate-mode) |
| windows | 0.52 | Safe Win32 API bindings |
| goblin | 0.8 | Pure-Rust PE parser |
| sha2 | 0.10 | SHA-256 hashing |
| hex | 0.4 | Hex encoding |
| rfd | 0.14 | Native file dialogs |
| chrono | 0.4 | Timestamp formatting |
| serde/bincode | 1.0/1.3 | Serialization |
| log/env_logger | 0.4/0.10 | Logging |
| rand | 0.8 | Random number generation |

---

## Building for Offline Environments

If crates.io is unavailable:

1. **Configure a local mirror** in `.cargo/config.toml`:
   ```toml
   [source.crates-io]
   replace-with = "mirror"
   
   [source.mirror]
   registry = "file:///path/to/local/registry"
   ```

2. **Or use prebuilt binary**: `/deps/psexec_rs.exe`

---

## Testing

### Manual Testing (GUI Mode)
```bash
cargo run --release
# Open a PE file (e.g., cmd.exe, notepad.exe)
# Verify timestamps display as readable dates
# Verify machine type shows "AMD64 (x64)" instead of "0x8664"
```

### Manual Testing (CLI Mode - Local)
```bash
# Execute local command
cargo run --release -- cmd /c "dir"
```

### Manual Testing (CLI Mode - Remote)
Requires:
- Two Windows machines on same domain
- Admin credentials for target machine
- Network connectivity and SMB access

```bash
cargo run --release -- \\targetmachine cmd /c "whoami"
```

---

## Security Considerations

- **Unsigned Binary**: Users may see SmartScreen warnings. Code sign the binary before distribution.
- **Plaintext Credentials**: CLI accepts passwords on command line (visible in process list). Consider environment variables or credential manager integration.
- **File Paths**: Analysis output includes full paths; sanitize before sharing logs.
- **Service Cleanup**: Orphaned services are cleaned up on startup, but manual verification is recommended.

---

## Deployment

### Distribution
- Single statically-linked .exe (~2.6 MB release build)
- No external DLLs required (all linked statically)
- Requires Windows x86-64, no specific .NET or runtime dependencies

### Permissions
- Local execution: requires user permissions for target process
- Remote execution: requires domain admin or equivalent credentials
- Service installation: requires admin privileges

---

## Contributing

Contributions are welcome! Before submitting, please:

1. Run `cargo build --release` and verify no warnings
2. Test on Windows x86-64
3. Follow Rust naming conventions (snake_case for functions/variables, PascalCase for types)
4. Include descriptive commit messages

---

## License

MIT License — see LICENSE file for details.

---

## References

- [Windows PE Format](https://learn.microsoft.com/en-us/windows/win32/debug/pe-format)
- [WinVerifyTrust API](https://learn.microsoft.com/en-us/windows/win32/api/wintrust/nf-wintrust-winverifytrust)
- [Service Control Manager](https://learn.microsoft.com/en-us/windows/win32/services/services)
- [egui Documentation](https://docs.rs/egui/0.27/)
- [goblin PE Parser](https://docs.rs/goblin/0.8/)
  
---

**Last Updated**: 2026-05-29  
**Status**: Active Development
