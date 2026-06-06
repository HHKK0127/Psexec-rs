# Phase 3.5 → Phase 4 への移行ガイド

**作成**: 2026-06-06  
**優先度**: 高  

---

## 🎯 即座に実装すべき項目（優先順）

### 1️⃣ GUI → CLI Handlers 接続【優先度: 最高】

**現状**:
```rust
// ui.rs の show_service_management() 内
if ui.button("▶ Refresh").clicked() {
    // 現在: モックデータ生成
    self.services = vec![...]; // sample data
}
```

**修正案**:
```rust
if ui.button("▶ Refresh").clicked() {
    // 実装: CLI handlers 呼び出し
    let cmd = ServiceCommands::List {
        host: Some(self.service_host.clone()),
        filter: None,
        running_only: false,
    };
    // cli_handlers::handle_service_command(Some(host), cmd).await
}
```

**課題**: UI スレッドでの `.await` は ブロッキング → 次項参照

---

### 2️⃣ 非同期実行【優先度: 最高】

**パターン**:
```rust
// ui.rs 内の TabState に mpsc チャネルを追加
pub struct AnalyzerApp {
    // ... existing fields
    service_cmd_rx: Option<std::sync::mpsc::Receiver<ServiceResult>>,
    service_cmd_tx: Option<std::sync::mpsc::Sender<ServiceCommand>>,
}

// Render ループ内
if let Ok(result) = self.service_cmd_rx.try_recv() {
    self.services = result.services;
    self.service_status_message = "Loaded".to_string();
}

// Button 押下時
if ui.button("▶ Refresh").clicked() {
    let host = self.service_host.clone();
    let tx = self.service_cmd_tx.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            cli_handlers::handle_service_command(Some(host), cmd).await
        });
        let _ = tx.send(result);
    });
}
```

**利点**: UI フリーズなし、バックグラウンド実行

---

### 3️⃣ エラー表示の改善【優先度: 高】

**現状**:
```rust
self.status_message = "Starting service: Windows Update";
// ← 成功・失敗を区別しない
```

**修正案**:
```rust
match result {
    Ok(_) => {
        self.status_message = "✅ Service started successfully".to_string();
        self.status_color = Color32::GREEN;
    }
    Err(e) => {
        self.status_message = format!("❌ Error: {}", e);
        self.status_color = Color32::RED;
    }
}
```

---

## 🧪 検証チェックリスト

### Service Tab
- [ ] `List` コマンド実行 → サービス一覧取得
- [ ] `Start` → サービス開始確認
- [ ] `Stop` → サービス停止確認
- [ ] `Delete` → サービス削除確認
- [ ] エラー表示: 存在しないサービス指定時

### Registry Tab
- [ ] `Read` → 値取得確認
- [ ] `Write` → 値更新確認 (REG_SZ, REG_DWORD)
- [ ] `Delete` → 値削除確認
- [ ] `List` → キー列挙確認
- [ ] エラー表示: アクセス拒否時

### Script Tab
- [ ] PowerShell スクリプト実行
- [ ] VBScript 実行
- [ ] Batch スクリプト実行
- [ ] 出力キャプチャ & 表示
- [ ] エラー: 無効なスクリプト型

### UI 応答性
- [ ] 100+ サービス リスト でフリーズなし
- [ ] 1MB スクリプト出力 でスクロール遅延なし
- [ ] リモートホスト接続時 タイムアウト処理

---

## 📝 修正が必要なファイル

| ファイル | 修正内容 | 優先度 |
|--------|--------|--------|
| `src/ui.rs` | mpsc チャネル追加、非同期呼び出し | 🔴 最高 |
| `src/cli_handlers.rs` | 戻り値型を構造化（ServiceList等）| 🔴 最高 |
| `src/gui/service_tab.rs` | エラーカラー表示、リトライ | 🟡 高 |
| `src/gui/registry_tab.rs` | REG_BINARY, REG_QWORD 対応 | 🟡 高 |
| `src/gui/script_tab.rs` | ファイルダイアログ (rfd) | 🟡 高 |

---

## 🚀 実装順序（推奨）

```
Step 1. CLI handlers 戻り値型設計 (30min)
   ├─ ServiceListResult { services: Vec<ServiceInfo> }
   ├─ RegistryResult { entries: Vec<RegistryEntry> }
   └─ ScriptResult { exit_code, stdout, stderr }

Step 2. UI mpsc チャネル統合 (1hr)
   ├─ AnalyzerApp に tx/rx 追加
   ├─ UI 側: try_recv() で結果確認
   └─ Button: std::thread::spawn で非同期実行

Step 3. 各タブでの実装 API 呼び出し (2-3hr)
   ├─ Service Tab: handle_service_command 統合
   ├─ Registry Tab: handle_registry_command 統合
   └─ Script Tab: handle_script_command 統合

Step 4. テスト & エラーハンドリング (1-2hr)
   ├─ 各API の成功ケース確認
   ├─ エラー時のメッセージ表示
   └─ UI フリーズテスト

Step 5. 最適化 & ドキュメント (1hr)
   ├─ パフォーマンス測定
   ├─ READMEの使用例追加
   └─ コミット & PR
```

**総見積もり**: 5-7 時間

---

## 💡 実装ヒント

### Service Context の初期化
```rust
let ctx = ServiceContext::new(&target_host);
let services = crate::service::list_services(&ctx).await?;
```

### Registry Context の初期化
```rust
let ctx = RegistryContext::new(
    &target_host,
    RegistryHive::HKEY_LOCAL_MACHINE
);
```

### Script 実行
```rust
let ctx = ScriptContext::new(ScriptType::PowerShell, &target_host);
let script = ScriptExecution::new(content).with_arguments(args);
let result = crate::script::execute_script(&ctx, &script).await?;
```

---

## 🔗 参考ファイル

- `src/cli_handlers.rs` — CLI 実装の参考（API呼び出しパターン）
- `CLAUDE.md` — プロジェクト概要（Module Responsibilities セクション）
- `README.md` — 使用例（CLI usage セクション）

---

**Next Session**: このファイルを読み込んで、GUI → CLI Handlers 接続作業を開始
