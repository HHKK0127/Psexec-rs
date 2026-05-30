# PAExec-rs

Rust で書かれた Windows リモートコマンド実行ツール。PsExec の Rust ポート版で、GUI PE ファイル分析、CLI リモート実行、Windows サービスエージェントの3つの操作モードをサポートしています。

![Rust](https://img.shields.io/badge/Rust-1.70+-red)
![Windows](https://img.shields.io/badge/Platform-Windows%20x86--64-blue)
![License](https://img.shields.io/badge/License-MIT-green)

## 機能

### 🔍 モード 1: GUI PE ファイル分析ツール
- ネイティブ Windows ファイルダイアログによるインタラクティブな PE ファイル分析
- メタデータ表示：ファイルサイズ、タイムスタンプ、SHA-256 ハッシュ
- PE ヘッダーの解析：マシン型、サブシステム、エントリーポイント、セクション
- インポートの抽出：DLL 名と関数名
- 文字列の抽出：バイナリから検出された ASCII/Unicode 文字列（キーワードフィルター付き）
- バージョン情報の取得：FileVersion、CompanyName、ProductName など
- Authenticode 署名検証（WinVerifyTrust 経由）
- 検索タブでリアルタイム フィルタリング

### 🖥️ モード 2: CLI リモート実行
- ローカルまたはリモート コンピューターでコマンドを実行
- PsExec 互換構文：`\\computername command args`
- 複数コンピューターへの並列実行
- 自動サービス インストール・クリーンアップ
- リモート ターゲットへのファイル転送
- 名前付きパイプによる通信プロトコル

### 🔧 モード 3: Windows サービス エージェント
- Windows サービスとして実行し、リモート コマンドを受信
- `\\.\pipe\PAExec-rs` でリスンする名前付きパイプ
- バイナリ メッセージのシリアライゼーション/デシリアライゼーション
- プロセス実行と終了コード報告
- 孤立したサービスの自動クリーンアップ

---

## インストール

### プリビルト バイナリ
プリビルト リリース バイナリは `/deps/psexec_rs.exe` で入手可能です（2.6 MB）。

### ソースからのビルド

**必要な環境:**
- Windows x86-64
- Rust 1.70+ (stable)
- インターネット接続（初回ビルド時のみ）

**ビルド コマンド:**

```bash
# デバッグビルド
cargo build

# リリースビルド（最適化版、約 2.6 MB）
cargo build --release

# 出力: target/release/psexec_rs.exe
```

---

## 使用方法

### GUI モード（PE ファイル分析ツール）

```bash
# GUI を起動（デフォルト、引数なし）
cargo run --release

# または明示的に指定
cargo run --release -- --gui
```

**機能:**
- 5つのタブ：概要、PE情報、インポート、文字列、署名
- 「ファイルを開く」をクリックして PE バイナリを選択
- インポートと文字列を検索フィールドでフィルタリング

### CLI モード（リモート実行）

```bash
# ローカルで実行
cargo run --release -- notepad.exe

# 単一のリモート コンピューターで実行
cargo run --release -- \\computername cmd /c "whoami"

# 複数のコンピューターで実行（カンマ区切り）
cargo run --release -- \\comp1,comp2,comp3 cmd /c "ipconfig"
```

**PsExec 互換構文:**
```
psexec [オプション] \\computer,computer2 application [引数]
```

**オプション:**
- `-u user` — ユーザー名を指定
- `-p password` — パスワードを指定
- `-c` — 実行前にリモートに実行ファイルをコピー
- `-f` — 強制的にコピー（既存ファイルを上書き）
- `-d` — プロセス終了を待たない
- `-t timeout` — タイムアウト（秒）

### サービス モード

```bash
# サービスとしてインストール・実行
cargo run --release -- -service
```

サービスは名前付きパイプでリッスンし、リモート クライアントから受信したコマンドを実行します。

---

## アーキテクチャ

### ディレクトリ構造

```
src/
├── main.rs              # エントリーポイント；ディスパッチャー（GUI/CLI/Service モード）
├── analyzer.rs          # PE ファイル解析
├── ui.rs                # egui/eframe GUI レンダリング
├── cli.rs               # コマンドライン引数解析
├── settings.rs          # 設定構造体
├── process.rs           # CreateProcessW ラッパー
├── remote.rs            # SMB/UNC パス処理
├── scm.rs               # サービス コントロール マネージャー（インストール/開始/停止）
├── pipes.rs             # 名前付きパイプ作成・メッセージング
├── proto.rs             # バイナリ メッセージ プロトコル
└── winapi_utils.rs      # Win32 API ヘルパー（バージョン情報、署名）
```

### 実行フロー

**GUI モード:**
1. ユーザーがネイティブダイアログからファイルを開く
2. `analyzer::analyze_file()` が PE を同期的に解析
3. 結果が egui タブに表示される

**CLI モード（リモート）:**
1. コマンドライン引数を解析
2. 各コンピューターに対して：管理共有に接続 → 実行ファイルをコピー → サービスをインストール → コマンドを実行
3. リモートで開始されたサービスがプロセスを実行し、終了コードを返す
4. クライアントが名前付きパイプ経由で結果を受け取る

**サービス モード:**
1. 名前付きパイプ リスナーを作成
2. クライアント接続をループで受け入れ
3. 各クライアント：設定をデシリアライズ → プロセスを実行 → 終了コードを返す

---

## 最近の変更

### バージョン 0.1.0（2026年5月29日）

**セキュリティ修正:**
- VerQueryValueW 解析でのバッファオーバーリード（未定義動作）を修正
- DoS/OOM 攻撃を防止するために 100MB ファイルサイズ制限を追加

**UX 改善:**
- PE タイムスタンプを人間が読める UTC 形式に変換（例："2025-01-01 12:00:00 UTC"）
- マシン型を可読形式に変換（i386、AMD64、ARM64 など）
- サブシステムを可読形式に変換（GUI、CUI、Native など）

**コード リファクタリング:**
- ファイルシステム メタデータの重複呼び出しを排除

---

## 既知の問題・制限

### 現在の状況
- ❌ ユニット テスト・統合テストなし
- ❌ CI/CD パイプラインなし
- ⚠️ バイナリは未署名（初回実行時に SmartScreen 警告）
- ⚠️ ネットワーク制限環境でのビルド（オフライン ビルドは失敗する可能性）

### UI スレッド ブロッキング
ファイル分析（ハッシング、PE 解析、Win32 呼び出し）が GUI スレッドをブロックします。大容量ファイル（1秒以上）は UI のフリーズを引き起こします。予定：分析をワーカー スレッドに移動。

### アーキテクチャに関する注記
- 同期 I/O のみ（async/await なし）
- シングル スレッド GUI（egui イミディエート モード）
- 証明書チェーンの抽出なし（署名検証のみ）

---

## 依存関係

| クレート | バージョン | 用途 |
|---------|---------|------|
| egui/eframe | 0.27 | GUI フレームワーク（イミディエート モード） |
| windows | 0.52 | 安全な Win32 API バインディング |
| goblin | 0.8 | 純粋 Rust PE パーサー |
| sha2 | 0.10 | SHA-256 ハッシング |
| hex | 0.4 | 16進数エンコーディング |
| rfd | 0.14 | ネイティブ ファイル ダイアログ |
| chrono | 0.4 | タイムスタンプ フォーマッティング |
| serde/bincode | 1.0/1.3 | シリアライゼーション |
| log/env_logger | 0.4/0.10 | ログ出力 |
| rand | 0.8 | 乱数生成 |

---

## オフライン環境でのビルド

crates.io が利用不可の場合：

1. **ローカル ミラーを設定** `.cargo/config.toml` に：
   ```toml
   [source.crates-io]
   replace-with = "mirror"
   
   [source.mirror]
   registry = "file:///path/to/local/registry"
   ```

2. **または プリビルト バイナリを使用**: `/deps/psexec_rs.exe`

---

## テスト

### 手動テスト（GUI モード）
```bash
cargo run --release
# PE ファイルを開く（例：cmd.exe、notepad.exe）
# タイムスタンプが可読日付で表示されることを確認
# マシン型が「0x8664」ではなく「AMD64 (x64)」と表示されることを確認
```

### 手動テスト（CLI モード - ローカル）
```bash
# ローカル コマンドを実行
cargo run --release -- cmd /c "dir"
```

### 手動テスト（CLI モード - リモート）
必要な条件：
- 同じドメイン上の 2 台の Windows マシン
- ターゲット マシンの管理者認証情報
- ネットワーク接続と SMB アクセス

```bash
cargo run --release -- \\targetmachine cmd /c "whoami"
```

---

## セキュリティに関する考慮事項

- **未署名バイナリ**: ユーザーは SmartScreen 警告を見る可能性があります。配布前にバイナリに署名してください。
- **プレーンテキスト認証情報**: CLI はコマンドライン上でパスワードを受け入れます（プロセス一覧で表示される）。環境変数または認証情報マネージャーの統合を検討してください。
- **ファイル パス**: 分析出力に完全なパスが含まれます。ログを共有する前に削除してください。
- **サービス クリーンアップ**: 孤立したサービスは起動時にクリーンアップされますが、手動による確認をお勧めします。

---

## デプロイメント

### 配布
- スタティック リンク済み単一 .exe（リリース ビルド約 2.6 MB）
- 外部 DLL 不要（すべてスタティック リンク）
- Windows x86-64 が必要、特定の .NET またはランタイム依存なし

### 権限
- ローカル実行：ターゲット プロセスの実行に必要なユーザー権限
- リモート実行：ドメイン管理者またはそれに相当する認証情報が必要
- サービス インストール：管理者権限が必要

---

## 貢献

貢献を歓迎します！提出前に、以下を確認してください：

1. `cargo build --release` を実行し、警告がないことを確認
2. Windows x86-64 でテスト
3. Rust 命名規約に従う（関数・変数は snake_case、型は PascalCase）
4. わかりやすいコミット メッセージを含める

---

## ライセンス

MIT ライセンス — LICENSE ファイルを参照してください。

---

## 参考資料

- [Windows PE 形式](https://learn.microsoft.com/ja-jp/windows/win32/debug/pe-format)
- [WinVerifyTrust API](https://learn.microsoft.com/ja-jp/windows/win32/api/wintrust/nf-wintrust-winverifytrust)
- [サービス コントロール マネージャー](https://learn.microsoft.com/ja-jp/windows/win32/services/services)
- [egui ドキュメント](https://docs.rs/egui/0.27/)
- [goblin PE パーサー](https://docs.rs/goblin/0.8/)

---

## 著者

小柄渕 寛樹（HHKK0127）  
メール: Hiroki.Kogarumai@protonmail.com

---

**最終更新**: 2026年5月29日  
**ステータス**: 活発な開発中
