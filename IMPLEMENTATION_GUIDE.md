# PAExec-rs 実装ガイド（Kimi向け）

**作成日時**: 2026-06-05

## 現在の実装状況

### ✅ 完了済み（Phase 1）

#### 1. 認証方式モジュール (`src/auth/mod.rs`)
- `AuthMethod` enum: CurrentUser, Credentials, NTHash, Kerberos
- `AuthContext` struct: 認証情報管理
- テスト: 4個 ✅

#### 2. 実行メソッドモジュール (`src/executor/mod.rs`)
- `ExecutionMethod` enum: SMBService, WMI, TaskScheduler, DCOM
- `ExecutionContext` struct: 実行設定管理
- `ExecutionResult` struct: 結果返却

#### 3. WMI実行メソッド (`src/executor/wmi.rs`)
- `execute_via_wmi()` 関数
- PowerShell コマンド自動生成
- ドメイン認証対応

#### 4. Task Scheduler実行メソッド (`src/executor/task_scheduler.rs`)
- `execute_via_task_scheduler()` 関数
- Demand / Create / Change モード
- 自動クリーンアップ

#### 5. エラーハンドリング (`src/error.rs`)
- `PaExecError` enum: 11個のエラーコード
- `RetryPolicy` struct: リトライ制御
- テスト: 複数 ✅

#### 6. Named Pipeプロトコル (`src/pipe/protocol.rs`)
- `Message` struct: メッセージシリアライゼーション
- `ExecutionSettings` struct: JSON ベース設定
- `ExecutionResult` struct: 結果構造体
- テスト: 複数 ✅

#### 7. インタラクティブシェル (`src/pipe/interactive.rs`)
- `InteractiveSession` struct: セッション管理
- コマンドキューイング
- 出力バッファ管理
- テスト: 複数 ✅

---

## Phase 2 実装タスク

### タスク 2.1: ファイル転送機能（双方向）

**目的**: ローカルとリモート間のファイルコピー実装

#### 2.1.1 ファイル転送モジュール作成

**ファイル名**: `src/file_transfer/mod.rs`

**要件**:
- SMB admin$ 共有経由のファイル転送
- アップロード機能: ローカル → リモート
- ダウンロード機能: リモート → ローカル
- フォルダ転送（再帰的）
- 大容量ファイル対応（ストリーム分割）

**実装する型/関数**:

```rust
pub enum TransferDirection {
    Upload,      // ローカル → リモート
    Download,    // リモート → ローカル
}

pub struct FileTransferContext {
    pub remote_host: String,
    pub auth: AuthContext,
    pub timeout_seconds: u32,
}

impl FileTransferContext {
    // 新規作成
    pub fn new(remote_host: &str, auth: AuthContext) -> Self
    
    // タイムアウト設定
    pub fn with_timeout(mut self, seconds: u32) -> Self
}

// 単一ファイル転送（同期版）
pub fn transfer_file(
    context: &FileTransferContext,
    local_path: &str,
    remote_path: &str,
    direction: TransferDirection,
) -> Result<TransferResult, PaExecError>

// 複数ファイル転送（バッチ）
pub fn transfer_files_batch(
    context: &FileTransferContext,
    transfers: Vec<(String, String, TransferDirection)>,
) -> Result<Vec<TransferResult>, PaExecError>

pub struct TransferResult {
    pub local_path: String,
    pub remote_path: String,
    pub bytes_transferred: u64,
    pub success: bool,
    pub error: Option<String>,
}
```

**実装戦略**:
1. PowerShell の `Copy-Item` を使用（Win32 API より簡潔）
2. UNC パス `\\server\admin$\` で admin$ 共有にアクセス
3. フォルダは再帰的にコピー（`-Recurse` フラグ）
4. 進捗トラッキングは転送結果で返却

**参考実装**:
- goexec: `pkg/goexec/smb/input.go`, `output.go`
- PAExec: `Remote.cpp` (CopyPAExecToRemote 関数)

**テスト必須**:
- ✓ 単一ファイルアップロード
- ✓ 単一ファイルダウンロード
- ✓ フォルダ転送（再帰）
- ✓ 権限エラーハンドリング

---

### タスク 2.2: 出力取得の複数方法対応

**目的**: 様々な環境での出力取得確保

#### 2.2.1 出力取得モジュール作成

**ファイル名**: `src/output/mod.rs`, `src/output/fetcher.rs`

**要件**:
- Named Pipe ベース出力取得（既存）
- SMB ファイル転送ベース出力取得（新規）
- リアルタイムストリーミング対応

**実装する型/関数**:

```rust
pub enum OutputMethod {
    NamedPipe,    // \.\pipe\RemComPIPENAME
    SMBFile,      // admin$ 共有経由でファイル読取
    DirectSocket, // 直接通信（オプション）
}

pub trait OutputFetcher {
    fn fetch_output(
        &self,
        pipe_name: &str,
        timeout_ms: u32,
    ) -> Result<String, PaExecError>;
    
    fn stream_output<F>(
        &self,
        pipe_name: &str,
        callback: F,
    ) -> Result<(), PaExecError>
    where
        F: Fn(String) -> ();
}

pub struct NamedPipeFetcher;
pub struct SMBFileFetcher;

impl OutputFetcher for NamedPipeFetcher {
    // Named Pipe から出力読取（既存ロジックをここに移す）
}

impl OutputFetcher for SMBFileFetcher {
    // SMB 共有からファイルを読取
}

pub fn create_fetcher(method: OutputMethod) -> Box<dyn OutputFetcher>
```

**実装戦略**:
1. SMBFileFetcher: リモートの `C:\Windows\Temp\paexec_output_[timestamp].txt` から読取
2. NamedPipeFetcher: 既存コードをリファクタして移す
3. タイムアウト: 30秒まで待機（Overlapped I/O 推奨）

**参考実装**:
- goexec: `pkg/goexec/output/`

---

### タスク 2.3: プロセス管理機能

**目的**: リモート PC のプロセス制御

#### 2.3.1 プロセス管理モジュール作成

**ファイル名**: `src/process/mod.rs`

**要件**:
- リモートプロセス一覧取得
- プロセス終了
- 優先度制御
- CPU アフィニティ設定

**実装する型/関数**:

```rust
pub struct RemoteProcess {
    pub pid: u32,
    pub name: String,
    pub memory_mb: u64,
    pub cpu_percent: f64,
}

pub struct ProcessManager {
    pub remote_host: String,
    pub auth: AuthContext,
}

impl ProcessManager {
    pub fn new(remote_host: &str, auth: AuthContext) -> Self
    
    // プロセス一覧取得（WMI）
    pub fn list_processes(&self) -> Result<Vec<RemoteProcess>, PaExecError>
    
    // プロセス終了
    pub fn kill_process(&self, pid: u32) -> Result<bool, PaExecError>
    
    // 優先度設定
    pub fn set_priority(&self, pid: u32, priority: u32) -> Result<bool, PaExecError>
}

pub enum ProcessPriority {
    Idle = 4,
    BelowNormal = 6,
    Normal = 8,
    AboveNormal = 10,
    High = 13,
    Realtime = 24,
}
```

**実装戦略**:
1. WMI: `Get-Process` PowerShell コマンドで一覧取得
2. 終了: `Stop-Process -Id [pid] -Force`
3. 優先度: `Set-ProcessPriority -ProcessId [pid] -Priority [level]`

**参考実装**:
- Quasar: process monitoring

---

### タスク 2.4: ログ・トレース機能強化

**目的**: デバッグ・監査対応

#### 2.4.1 ログモジュール作成

**ファイル名**: `src/logging/mod.rs`

**要件**:
- 詳細エラーログ
- JSON 形式ログ出力
- ファイルログ出力
- タイムスタンプ自動付与

**実装する関数**:

```rust
pub fn init_logging(
    log_file: Option<&str>,
    json_format: bool,
) -> Result<(), PaExecError>

pub fn log_execution(
    target: &str,
    command: &str,
    result: &ExecutionResult,
) -> Result<(), PaExecError>

pub fn log_connection_attempt(
    target: &str,
    method: ExecutionMethod,
    success: bool,
) -> Result<(), PaExecError>
```

**実装戦略**:
1. `env_logger` + `serde_json` を使用
2. RUST_LOG 環境変数で制御（既に設定可能）
3. ファイル出力: `~/.paexec/logs/paexec-[date].log`

---

### タスク 2.5: 並列実行・バッチ処理

**目的**: 複数 PC への効率的な実行

#### 2.5.1 バッチ実行モジュール作成

**ファイル名**: `src/batch/mod.rs`

**要件**:
- 複数コンピュータへの並列実行
- 実行結果の統合表示
- エラーハンドリング（1台失敗しても継続）

**実装する型/関数**:

```rust
pub struct BatchExecutionConfig {
    pub targets: Vec<String>,        // コンピュータ一覧
    pub command: String,             // 実行コマンド
    pub auth: AuthContext,
    pub max_parallel: usize,         // 並列数（デフォルト4）
    pub timeout_seconds: u32,
}

pub struct BatchResult {
    pub target: String,
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub async fn execute_batch(
    config: BatchExecutionConfig,
) -> Result<Vec<BatchResult>, PaExecError>
```

**実装戦略**:
1. `tokio::spawn` で並列実行
2. `futures::stream::FuturesUnordered` で管理
3. 結果は Vec で集約

**参考実装**:
- goexec: cmd/batch.go

---

## 実装スケジュール推奨

| 優先順位 | タスク | 難易度 | 推定期間 |
|---------|--------|--------|---------|
| 1 | 2.1 ファイル転送 | 中 | 2-3日 |
| 2 | 2.2 出力取得 | 中 | 1-2日 |
| 3 | 2.3 プロセス管理 | 低 | 1日 |
| 4 | 2.4 ログ強化 | 低 | 1日 |
| 5 | 2.5 バッチ処理 | 中 | 2日 |

---

## 共通実装ガイドライン

### コード品質基準
- ✓ すべての pub 関数にテストを記述
- ✓ エラーハンドリングで `PaExecError` を使用
- ✓ コメントは「なぜ」を説明（「何」ではなく）
- ✓ 200文字以上のコメントなし
- ✓ 不要な抽象化は行わない

### ファイル構成
- `src/[module]/mod.rs` ← 公開インターフェース
- `src/[module]/internal.rs` ← 内部実装（private）
- テストは同じファイルの `#[cfg(test)]` に記述

### 依存関係
既に導入済み:
- `windows 0.52` — Win32 API
- `tokio 1.35` — 非同期実行
- `serde_json 1.0` — JSON
- `uuid 1.6` — ID生成
- `encoding_rs 0.8` — 文字コード変換
- `chardet 0.2` — 文字コード検出

新規追加が必要な場合は、Kimi → Claude に相談してください。

---

## Kimi への質問テンプレート

実装中に不明な点がある場合、以下の形式で質問してください：

```
【質問】[タスク番号] [内容]
【背景】[なぜこれが必要か]
【選択肢】
A) [案1]
B) [案2]
【参考】[関連ファイル/コード片]
```

---

**次の実装**: タスク 2.1 ファイル転送機能 (2026-06-05)

