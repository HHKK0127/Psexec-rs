# PAExec-rs 実装メモリ

## セッション日時
2026-06-05

## プロジェクト目標
PAExec（PsExec クローン）の Rust 実装。以下の機能を段階実装。

## 導入機能一覧（18個）

### Phase 1：コア機能（4-6週間）
1. **複数認証方式** - NTLM、Kerberos、NTハッシュ、SPNEGO
2. **WMI実行メソッド** - Win32_Process.Create
3. **Task Scheduler実行** - demand/create/change方式
4. **DCOM実行メソッド** - MMC20.Application（オプション）
5. **インタラクティブシェル** - 複数コマンド連続実行
6. **エラーハンドリング強化** - 詳細エラーコード、リトライロジック

### Phase 2：管理機能（3-4週間）
7. **ファイル転送（双方向）** - ローカル↔リモート
8. **出力取得の複数化** - Named Pipe、SMBファイル、ストリーミング
9. **プロセス管理** - 一覧表示、優先度制御、終了
10. **ログ・トレース強化** - JSON形式、ファイル出力
11. **並列実行・バッチ処理** - tokio::spawn対応

### Phase 3：高度な機能（4-5週間）
12. **レジストリ操作** - 読取、書込、作成、削除
13. **サービス管理** - 一覧、開始、停止、作成
14. **スクリプト実行** - PowerShell、VBScript、バッチ
15. **GUI操作機能** - スクリーンショット、マウス、キーボード（オプション）

### Phase 4：セキュリティテスト機能（2-3週間・オプション）
16. **キーロガー機能** - キー入力記録
17. **DLL/Shellcodeインジェクション** - メモリ内実行
18. **ネットワーク機能** - SOCKS5、ポートフォワーディング

## 参考プロジェクトの場所
Z:\Other\Psexec\psexec-sources\

### 各プロジェクト役割
- **goexec** - 複数実行方法、認証方式の参考（Go言語）
- **pypsexec** - Named Pipe通信、出力取得の参考（Python、読みやすい）
- **CSExec** - インタラクティブシェル実装の参考（C#、シンプル）
- **PAExec** - エラーハンドリング、Overlapped I/Oの参考（C++）
- **RemCom** - リモート接続フロー、サービス管理の参考（C++）
- **Quasar** - GUI機能、プロセス管理の参考
- **mRemoteNG** - マルチプロトコル対応の参考
- **DuplexSpyCS** - 高度な機能（キーロガー、インジェクション）の参考

## Rust実装ライブラリ構成（推奨）
```toml
[dependencies]
windows = "0.52"              # Win32 API
gssapi = "*"                  # Kerberos/NTLM/SPNEGO
ntlm-rs = "*"                 # NTLM認証（フォールバック）
tokio = { version = "*", features = ["full"] }  # 非同期実行
serde = { version = "1.0", features = ["derive"] }  # シリアライズ
serde_json = "1.0"           # JSON形式ログ
encoding_rs = "0.8"          # 文字エンコーディング（既に導入済み）
chardet = "0.2"              # 文字コード検出（既に導入済み）
```

## 実装ファイル構成（予定）
```
src/
├── main.rs                   # CLI/GUI エントリーポイント
├── lib.rs                    # ライブラリ公開インターフェース
├── auth/
│   ├── mod.rs               # AuthMethod enum + GSSAPI実装
│   ├── ntlm.rs              # NTLM フォールバック
│   └── kerberos.rs          # Kerberos実装
├── executor/
│   ├── mod.rs               # RemoteExecutor trait
│   ├── smb_service.rs       # 現在の実装（リファクタ）
│   ├── wmi.rs               # WMI実行メソッド
│   ├── task_scheduler.rs    # Task Scheduler
│   └── dcom.rs              # DCOM（Phase 3）
├── pipe/
│   ├── mod.rs               # Named Pipe ラッパー
│   ├── protocol.rs          # メッセージプロトコル
│   └── interactive.rs       # インタラクティブシェル
├── output/
│   ├── mod.rs               # OutputFetcher trait
│   ├── pipe.rs              # Named Pipe ベース
│   └── smb.rs               # SMB ファイル転送
├── file_transfer/
│   ├── mod.rs               # ファイル転送実装
│   └── smb.rs               # SMB経由の転送
├── process/
│   ├── mod.rs               # プロセス管理
│   └── remote.rs            # リモートプロセス制御
├── registry/
│   ├── mod.rs               # レジストリ操作（Phase 3）
│   └── remote.rs            # リモートレジストリ
├── service/
│   ├── mod.rs               # Windows サービス管理
│   └── remote.rs            # リモートサービス制御
└── error.rs                  # エラー定義
```

## 核となるAPI・RPC仕様

### WMI実行メソッド
- **RPC Endpoint**: DCOM COM v5.7
- **WMI Class**: Win32_Process
- **Method**: Create
- **Parameters**: CommandLine, WorkingDirectory, Priority, Affinity
- **Return**: ProcessId, ReturnValue
- **参考**: goexec/cmd/wmi.go, goexec/pkg/goexec/wmi/proc.go

### Task Scheduler実行メソッド
- **RPC Endpoint**: ncacn_np:[atsvc]
- **Methods**: SchRpcRegisterTask, SchRpcRun, SchRpcRetrieveTask, SchRpcRevert, SchRpcDelete
- **タスク設定**: TimeTrigger, DeleteExpiredTaskAfter, SessionId, UserSID
- **参考**: goexec/cmd/tsch.go, goexec/pkg/goexec/tsch/

### Named Pipeプロトコル
- **パイプ名パターン**: `\.\pipe\RemComPIPENAME` または `\.\pipe\PaExecXXXX`
- **パイプモード**: PIPE_READMODE_MESSAGE（メッセージ境界検出）
- **I/O方式**: Overlapped I/O（非ブロッキング）
- **メッセージ構造**: [Length: u32][Data: Vec<u8>]
- **参考**: CSExec/csexec/Client.cs, RemCom/RemCom.cpp, PAExec/Remote.cpp

### 認証フロー
- **GSSAPI**: SPNEGO → NTLM → Kerberos（自動フォールバック）
- **NTハッシュ**: 直接使用（パスワード不要）
- **Kerberos**: DC指定対応、キャッシュ対応
- **参考**: goexec/cmd/root.go (lines 226-264)

### エラーハンドリング
- **エラーコード体系**: -1～-11（詳細分類）
- **リトライロジック**: 最大20回、1秒間隔
- **タイムアウト**: 接続20秒、実行別途設定
- **参考**: PAExec/Remote.cpp (lines 438-446), RemCom/RemCom.cpp (lines 241-262)

## 現在の実装状況（基準日：2026-06-05）
- ✅ GUI PE ファイルアナライザー（完成）
- ✅ CLI リモートコマンド実行（PowerShell Remoting ベース）
- ✅ 文字化け自動修正（chardet + encoding_rs）
- ⚠️ 認証方式：現在のユーザーのみ
- ⚠️ 実行方法：PowerShell Remoting のみ
- ❌ インタラクティブシェル
- ❌ ファイル転送
- ❌ プロセス管理
- ❌ レジストリ操作
- ❌ サービス管理

## 実装優先度マトリックス
🔴 高効果・低難易度（最優先）:
- 複数認証方式 ⭐⭐⭐
- WMI実行メソッド ⭐⭐⭐
- インタラクティブシェル ⭐⭐⭐

🟡 高効果・高難易度（優先実装）:
- Task Scheduler実行 ⭐⭐⭐
- ファイル転送 ⭐⭐
- エラーハンドリング ⭐⭐

## 重要な制約・注意点
- GUI実装はユーザーが別途対応（処理部分のRustのみ実装）
- 既存コードとの互換性を保つ
- 文字化け自動修正は維持
- Phase 1 完了で goexec の60%機能相当を達成
