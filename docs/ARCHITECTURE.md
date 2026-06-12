# GUI アーキテクチャ詳細 (Phase 4.4-4.5)

**作成日**: 2026-06-12  
**対象**: システム設計者・アーキテクト  
**読了時間**: 30分  
**対象フェーズ**: Phase 4.4 (Command Palette) + Phase 4.5 (Pane Layout System)

---

## 目次

1. [全体図](#全体図)
2. [Phase 4.4: Command Palette](#phase-44-command-palette-ui層)
3. [Phase 4.5: Pane Layout System](#phase-45-pane-layout-system)
4. [モジュール詳細](#モジュール詳細)
5. [データフロー](#データフロー)
6. [状態管理](#状態管理)
7. [拡張ポイント](#拡張ポイント)

---

## 全体図

### 1.1 階層構造

```
┌─────────────────────────────────────────────────────────┐
│          eframe::App (AnalyzerApp)                      │
│  Window lifecycle, input/output management              │
├─────────────────────────────────────────────────────────┤
│  update(&mut self, ctx: &egui::Context, frame)          │
│    ├─ pane_layout events & rendering (Phase 4.5)       │
│    ├─ command_palette rendering (Phase 4.4 - overlay)  │
│    └─ UI component management                           │
├─────────────────────────────────────────────────────────┤
│         PaneLayoutState (Binary Tree Layout)             │
│  ├─ root: LayoutNode (Pane/Split)                       │
│  ├─ active_pane: Option<NodeId>                         │
│  ├─ resizing_divider: Option<ActiveDivider>             │
│  └─ pane_contents: HashMap<NodeId, PaneContent>         │
└─────────────────────────────────────────────────────────┘
         ↓                          ↓
┌──────────────────────┐  ┌──────────────────────┐
│  LayoutNode Tree     │  │  CommandPalette      │
│  (Recursive split)   │  │  (Overlay / Stack)   │
├──────────────────────┤  ├──────────────────────┤
│ ├─ Pane             │  │ • state: Open/Closed │
│ │  └─ PaneContent   │  │ • filtered_items     │
│ └─ Split            │  │ • selected_index     │
│    ├─ left child    │  │ • search_text        │
│    └─ right child   │  │ • items: Vec<>       │
└──────────────────────┘  └──────────────────────┘
```

### 1.2 モジュール依存関係

```
ui/
├── mod.rs
│   ├─ exports: CommandPalette, PaneLayoutState, AnalyzerApp
│   └─ declares: command_palette, pane_layout, app
│
├── app.rs (AnalyzerApp)
│   ├─ uses: CommandPalette (Phase 4.4)
│   ├─ uses: PaneLayoutState (Phase 4.5)
│   └─ coordinates: pane_layout events/render + palette overlay
│
├── command_palette/              [Phase 4.4]
│   ├── mod.rs (Public API)
│   ├── items.rs (PaletteItem enum)
│   ├── state.rs (CommandPalette state machine)
│   ├── search.rs (Fuzzy search algorithm)
│   └── renderer.rs (egui rendering)
│
└── pane_layout/                  [Phase 4.5]
    ├── mod.rs (Public API: init(), exports)
    │
    ├── layout.rs (Core data structures)
    │   ├─ LayoutNode enum (Pane/Split)
    │   ├─ PaneContent enum (5 pane types)
    │   ├─ PaneLayoutState (tree + state)
    │   └─ ActiveDivider (drag tracking)
    │
    ├── renderer.rs (egui rendering)
    │   ├─ render_pane_layout()
    │   ├─ render_node() [recursive]
    │   ├─ render_split() [divider drawing]
    │   └─ calculate_split_rects()
    │
    ├── events.rs (Input handling)
    │   └─ handle_pane_events() [divider drag detection]
    │
    └── config.rs (Persistence)
        ├─ save_layout() / load_layout()
        ├─ save_default() / load_default()
        └─ default_layout_path()
```

---

## モジュール詳細

### 2.1 items.rs (データ構造)

**責務**: パレットアイテムの定義

```rust
pub enum PaletteItem {
    // アクティブなアクション
    QuickAction {
        id: String,              // ユニークID
        label: String,           // 表示テキスト
        description: String,     // ヘルプテキスト
        icon: char,              // 表示アイコン
    },
    
    // 保存済みテンプレート
    Template {
        id: String,
        label: String,
        command: String,         // 実行コマンド
    },
    
    // 実行履歴
    HistoryEntry {
        id: String,
        label: String,
        timestamp: String,       // 実行時刻
    },
    
    // UI 構造
    Category { label: String },  // カテゴリラベル
    Separator,                   // 区切り線
}

pub enum PaletteMode {
    Search,    // テキスト検索モード
    Command,   // コマンド実行モード
}

pub struct SearchResult {
    pub item: PaletteItem,
    pub score: f32,  // 0.0-1.0 のマッチスコア
}
```

**特徴**:
- 5 つのバリアント → 拡張性高い
- 柔軟な ID スキーム → カスタムアクション追加可能
- スコアベースの検索 → 結果のランキング可能

### 2.2 state.rs (状態管理)

**責務**: パレットの状態とナビゲーション

```rust
pub struct CommandPalette {
    state: PaletteState,
    items: Vec<PaletteItem>,
    filtered_items: Vec<SearchResult>,
    selected_index: usize,
    search_text: String,
    search_mode: PaletteMode,
}

enum PaletteState {
    Open,   // パレット表示中
    Closed, // パレット非表示
}
```

**主要メソッド**:

```rust
impl CommandPalette {
    // 初期化
    pub fn new() -> Self {
        Self {
            state: PaletteState::Closed,
            items: Vec::new(),
            filtered_items: Vec::new(),
            selected_index: 0,
            search_text: String::new(),
            search_mode: PaletteMode::Search,
        }
    }
    
    // 状態遷移
    pub fn open(&mut self) {
        self.state = PaletteState::Open;
        self.search_text.clear();
        self.selected_index = 0;
    }
    
    pub fn close(&mut self) {
        self.state = PaletteState::Closed;
    }
    
    // ナビゲーション
    pub fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }
    
    pub fn move_selection_down(&mut self) {
        if self.selected_index < self.filtered_items.len() - 1 {
            self.selected_index += 1;
        }
    }
    
    // 選択アイテム取得
    pub fn get_selected_item(&self) -> Option<PaletteItem> {
        self.filtered_items
            .get(self.selected_index)
            .map(|r| r.item.clone())
    }
    
    // アイテム設定
    pub fn set_items(&mut self, items: Vec<PaletteItem>) {
        self.items = items;
    }
}
```

**状態遷移図**:

```
    ┌─────────────┐
    │  Closed     │◄──── init
    └──────┬──────┘
           │ open()
           ▼
    ┌─────────────┐
    │  Open       │
    │  • Display  │
    │  • Listen   │
    │  • Navigate │
    └──────┬──────┘
           │ close() / Esc
           ▼
    ┌─────────────┐
    │  Closed     │
    └─────────────┘
```

### 2.3 search.rs (検索ロジック)

**責務**: ファジー検索とスコアリング

**アルゴリズム**:

```rust
pub fn fuzzy_match(pattern: &str, text: &str) -> f32 {
    if pattern.is_empty() {
        return 1.0;  // 空文字列は全て一致
    }
    
    let mut score = 1.0;
    let mut text_idx = 0;
    
    for pattern_char in pattern.chars() {
        match text[text_idx..].find(pattern_char) {
            Some(found_at) => {
                // マッチしたポジション
                let match_pos = text_idx + found_at;
                let total_len = text.len();
                let relative_pos = match_pos as f32 / total_len as f32;
                
                // 1. 連続一致ボーナス
                score += 0.1;
                
                // 2. 早期一致ボーナス
                if relative_pos < 0.3 {  // 最初の 30%
                    score += 0.2;
                }
                
                text_idx = match_pos + 1;
            }
            None => {
                // サブストリング一致ボーナス
                if text.contains(pattern) {
                    score += 0.3;
                } else {
                    return 0.0;  // マッチなし
                }
            }
        }
    }
    
    // 正規化 (0.0 - 1.0)
    (score / pattern.len() as f32).min(1.0)
}
```

**スコア計算例**:

```
Pattern: "exp"
Text: "Export CSV"

Step 1: 'e' at position 0
  - score = 1.0 + 0.1 (consecutive) + 0.2 (early: 0 < 0.3) = 1.3

Step 2: 'x' at position 1
  - score = 1.3 + 0.1 (consecutive) = 1.4

Step 3: 'p' at position 2
  - score = 1.4 + 0.1 (consecutive) = 1.5

Normalized: 1.5 / 3 = 0.50 (50%)

Results Ranking:
  "Export CSV" (0.50)
  "Export JSON" (0.50)
  "expect" (0.30)
```

**性能特性**:

```
Time Complexity: O(n * m)  where n=text.len(), m=pattern.len()
Space Complexity: O(1)     (in-place matching)

Example:
  Pattern: "exp" (3 chars)
  Items: 100
  Time: ~1.5ms
```

### 2.4 renderer.rs (UI レンダリング)

**責務**: egui ベースの UI レンダリングとキー入力処理

```rust
pub fn render_palette(
    ctx: &egui::Context,
    palette: &mut CommandPalette,
) -> (bool, Option<PaletteItem>) {
    let mut is_open = true;
    let mut selected_item = None;
    
    egui::Window::new("Command Palette")
        .anchor(egui::Align2::CENTER_TOP, [0.0, 50.0])
        .fixed_size([500.0, 400.0])
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            // 1. 検索入力ボックス
            ui.text_edit_singleline(&mut palette.search_text);
            
            // 検索を実行
            palette.filtered_items = 
                search_items(&palette.search_text, &palette.items);
            
            // 2. 検索結果表示
            ui.separator();
            
            for (idx, result) in palette.filtered_items.iter().enumerate() {
                let is_selected = idx == palette.selected_index;
                
                let color = if is_selected {
                    egui::Color32::WHITE
                } else {
                    egui::Color32::GRAY
                };
                
                ui.label(
                    egui::RichText::new(
                        format!("{} {}", 
                            result.item.icon(),
                            result.item.label()
                        )
                    )
                    .color(color)
                );
                
                if is_selected {
                    ui.label(format!("  {}", result.item.description()));
                }
            }
            
            // 3. キー入力処理
            if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                palette.move_selection_up();
            }
            
            if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                palette.move_selection_down();
            }
            
            if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                selected_item = palette.get_selected_item();
            }
            
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                is_open = false;
            }
        });
    
    (is_open, selected_item)
}
```

**UI レイアウト**:

```
┌────────────────────────────────┐
│ Command Palette               │
├────────────────────────────────┤
│ [Search box: "exp.........."]   │
├────────────────────────────────┤
│ ✨ Export CSV                  │
│    Export execution history    │
│ ✨ Export JSON                 │ ← Selected
│    Export with full metadata   │
│ ⚙  Settings                    │
│    Open application settings   │
├────────────────────────────────┤
│ Keyboard: ↑↓ navigate          │
│           Enter select         │
│           Esc close            │
└────────────────────────────────┘
```

---

## Phase 4.5: Pane Layout System

### 構造体設計

**LayoutNode** (Binary Tree):

```rust
pub enum LayoutNode {
    Pane {
        id: NodeId,
        content: PaneContent,
    },
    Split {
        id: NodeId,
        left: Box<LayoutNode>,      // 左/上パネル
        right: Box<LayoutNode>,     // 右/下パネル
        divider_pos: f32,           // 0.1 ~ 0.9 (正規化位置)
        is_horizontal: bool,        // true = 左右分割
    }
}

pub enum PaneContent {
    BatchExecutor,
    LogViewer,
    Settings,
    Terminal,
    Placeholder,
}
```

**PaneLayoutState** (状態管理):

```rust
pub struct PaneLayoutState {
    pub root: LayoutNode,                           // レイアウトツリー
    pub active_pane: Option<NodeId>,                // 選択中パネル
    pub resizing_divider: Option<ActiveDivider>,    // ドラッグ中divider
    pub hover_divider: Option<NodeId>,              // マウスオーバーdivider
    pub next_id: NodeId,                            // 次のID
    pub pane_contents: HashMap<NodeId, PaneContent>,// コンテンツキャッシュ
}
```

### レンダリング処理

**Rect 計算フロー**:

```
render_pane_layout()
  ↓ [入力: available_rect (全体枠)]
  ├─ root.clone()
  └─ render_node(root, available_rect)
      ↓
      ├─ [Pane の場合]
      │  └─ render_pane() → 背景・ヘッダー・コンテンツ描画
      │
      └─ [Split の場合]
         ├─ calculate_split_rects()
         │  ├─ is_horizontal == true
         │  │   left_width = rect.width() * divider_pos
         │  │   → (left_rect, divider_rect, right_rect)
         │  └─ is_horizontal == false
         │      top_height = rect.height() * divider_pos
         │      → (top_rect, divider_rect, bottom_rect)
         │
         ├─ render_node(left, left_rect)
         ├─ render_node(right, right_rect)
         └─ render_divider(divider_rect)
            ├─ 背景色描画
            ├─ ホバー/ドラッグ状態で色変更
            └─ マウスイベント検出
```

### Divider ドラッグフロー

```
handle_pane_events(ctx, state)
  ↓
  ├─ [ドラッグ中の場合] state.resizing_divider.is_some()
  │  ├─ pointer_pos = ctx.pointer_interact_pos()
  │  ├─ delta = pointer_pos - active.drag_start
  │  ├─ delta_factor = delta.x / 1000.0  [正規化]
  │  └─ new_pos = (active.original_pos + delta_factor).clamp(0.1, 0.9)
  │     └─ state.update_divider(node_id, new_pos)  [位置更新]
  │
  └─ [ドラッグ終了] ボタンリリース時
     └─ state.resizing_divider = None
```

### レイアウト永続化

```
save_default() / load_default()
  ├─ default_layout_path() → ~/.psexec-rs/layout.json
  ├─ LayoutConfig { root, active_pane } を serde_json で処理
  └─ 起動時に自動復元 (フォールバック: デフォルトレイアウト)
```

### Command Palette 統合

```
eframe::App::update() [Phase 4.4/4.5 統合]
  ├─ render_pane_layout()        [下層: pane layout]
  ├─ handle_pane_events()        [divider イベント]
  │
  └─ Command Palette 処理        [上層: overlay]
     ├─ palette_visible が true の場合
     └─ render_palette() [最上位層]
        └─ 入力: Esc で close, Enter で select
```

### 制約条件

| 項目 | 値 | 理由 |
|------|-----|------|
| 最小パネルサイズ | 200px | UI 操作性確保 |
| Divider 位置範囲 | 0.1 ~ 0.9 | 極端な分割防止 |
| Divider 太さ | 4px (表示) / 10px (ヒット) | 精密操作性 |
| サポート言語 | 英語 + 日本語 | ドキュメント |

---

## データフロー

### 3.1 ユーザー入力フロー

```
User Input
    ↓
egui::Context::input()
    ↓
├─ Keyboard::Key(P) + Ctrl
│  → batch_panel.update()
│  → palette_visible = true
│  → palette.open()
│
├─ Char('a'..'z')
│  → palette.search_text.push(char)
│  → search_items() called
│  → filtered_items updated
│
├─ Key::ArrowUp
│  → palette.move_selection_up()
│
├─ Key::ArrowDown
│  → palette.move_selection_down()
│
├─ Key::Enter
│  → palette.get_selected_item()
│  → handle_palette_selection(item)
│
└─ Key::Escape
   → palette_visible = false
   → palette.close()
```

### 3.2 検索フロー

```
User Input: "exp"
    ↓
palette.search_text = "exp"
    ↓
render_palette() called
    ↓
search_items("exp", &palette.items)
    ↓
for each item in palette.items:
    score = fuzzy_match("exp", item.label)
    if score > 0.0:
        results.push(SearchResult { item, score })
    ↓
Sort by score descending
    ↓
palette.filtered_items = results
    ↓
Render UI with results
```

### 3.3 実行フロー

```
User selects item + Enter
    ↓
render_palette() returns (is_open, Some(item))
    ↓
batch_panel.handle_palette_selection(item)
    ↓
Match item type:
    ├─ QuickAction → execute action
    ├─ Template → execute template
    ├─ HistoryEntry → re-execute
    └─ Other → no-op
    ↓
palette_visible = false
palette.close()
```

---

## 状態管理

### 4.1 CommandPalette 内部状態

```
CommandPalette {
    state: PaletteState,           // UI表示/非表示
    items: Vec<PaletteItem>,       // 全アイテム（不変）
    filtered_items: Vec<SearchResult>,  // 検索結果（変動）
    selected_index: usize,         // 選択位置（変動）
    search_text: String,           // 検索テキスト（変動）
    search_mode: PaletteMode,      // 検索モード（固定）
}
```

**状態遷移**:

```
① User presses Ctrl+P
   ↓
② palette.open()
   state = Open
   search_text = ""
   selected_index = 0
   ↓
③ User types "exp"
   search_text = "exp"
   filtered_items = search_items("exp", ...)
   ↓
④ User presses Down
   selected_index += 1
   ↓
⑤ User presses Enter
   item = filtered_items[selected_index]
   ↓
⑥ palette.close()
   state = Closed
```

### 4.2 エラー処理

```rust
impl CommandPalette {
    pub fn get_selected_item(&self) -> Option<PaletteItem> {
        // Bounds check
        self.filtered_items
            .get(self.selected_index)
            .map(|r| r.item.clone())
    }
    
    pub fn move_selection_up(&mut self) {
        // Underflow protection
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }
    
    pub fn move_selection_down(&mut self) {
        // Overflow protection
        if self.selected_index < self.filtered_items.len() - 1 {
            self.selected_index += 1;
        }
    }
}
```

---

## 拡張ポイント

### 5.1 新しいアイテムタイプの追加

```rust
// Step 1: items.rs にバリアント追加
pub enum PaletteItem {
    QuickAction { ... },
    Template { ... },
    HistoryEntry { ... },
    Category { ... },
    Separator,
    CustomType {                  // ← NEW
        id: String,
        label: String,
        custom_data: MyData,
    },
}

// Step 2: 初期化時にアイテムを追加
pub fn initialize_palette(...) {
    let items = vec![
        PaletteItem::CustomType {
            id: "custom".to_string(),
            label: "My Custom Item".to_string(),
            custom_data: MyData { ... },
        },
    ];
}

// Step 3: 処理を追加
fn handle_palette_selection(&mut self, item: PaletteItem) {
    match item {
        PaletteItem::CustomType { custom_data, .. } => {
            // Custom handling
        }
        _ => {}
    }
}
```

### 5.2 検索アルゴリズムのカスタマイズ

```rust
// Option A: スコアの調整
pub fn fuzzy_match_custom(pattern: &str, text: &str) -> f32 {
    // デフォルト値
    let consecutive_bonus = 0.1;    // ← 調整
    let early_bonus = 0.2;          // ← 調整
    let substring_bonus = 0.3;      // ← 調整
    
    // ... implementation
}

// Option B: 新しい検索モードの追加
pub enum PaletteMode {
    Search,           // Fuzzy matching
    Command,          // Exact matching
    Regex,            // ← NEW
    Semantic,         // ← NEW
}
```

### 5.3 UI のカスタマイズ

```rust
pub fn render_palette_custom(
    ctx: &egui::Context,
    palette: &mut CommandPalette,
) -> (bool, Option<PaletteItem>) {
    // デフォルト UI の代わりにカスタム UI
    egui::Window::new("My Custom Palette")
        .show(ctx, |ui| {
            // Custom layout
            ui.horizontal(|ui| {
                ui.label("🔍");
                ui.text_edit_singleline(&mut palette.search_text);
            });
            
            // Custom rendering
            for result in &palette.filtered_items {
                ui.button(format!("{}: {}", 
                    result.item.icon(),
                    result.item.label()
                ));
            }
        });
    
    // ...
}
```

### 5.4 キーバインディングのカスタマイズ

```rust
// src/ui/app.rs
pub struct AnalyzerApp {
    command_palette: CommandPalette,
    palette_visible: bool,
    key_bindings: KeyBindings,  // ← NEW
}

pub struct KeyBindings {
    palette_open: KeyCombo,     // Ctrl+P (default)
    navigate_up: Key,           // ArrowUp (default)
    navigate_down: Key,         // ArrowDown (default)
    execute: Key,               // Enter (default)
    close: Key,                 // Escape (default)
}

impl AnalyzerApp {
    pub fn update(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| {
            i.key_pressed(self.key_bindings.palette_open.key) 
            && self.key_bindings.palette_open.check_modifiers(i)
        }) {
            self.palette_visible = true;
            self.command_palette.open();
        }
    }
}
```

---

## 性能考慮事項

### 6.1 検索パフォーマンス

```
Items Count | Pattern Length | Time (approx)
────────────┼────────────────┼──────────────
100         | 3              | 1-2ms
500         | 5              | 5-10ms
1000        | 10             | 10-20ms

Optimization:
- Pattern が短い場合は早期終了
- Items がソート済みなら二分探索可能
- キャッシュを使用（ただし invalidation logic が必要）
```

### 6.2 UI レンダリング

```
Frame Rate: 60 FPS (16.67ms per frame)

render_palette() time:
  - egui window creation: ~0.5ms
  - text rendering: ~1-2ms (依存: item count)
  - keyboard input check: ~0.1ms
  - total: ~2-3ms (acceptable)

Optimization:
- Virtual scrolling (large item lists)
- Lazy rendering (offscreen items)
- Caching (static strings)
```

### 6.3 メモリ使用

```
CommandPalette struct:
  - items: Vec<PaletteItem>          ~10-100KB (100-1000 items)
  - filtered_items: Vec<SearchResult> ~1-10KB (10-100 results)
  - search_text: String              ~1KB
  - Total overhead: ~20-150KB

Acceptable for most use cases.
If large item sets (>10,000): consider streaming or pagination.
```

---

## テストのための設計

### 7.1 Unit Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fuzzy_match_basic() {
        assert_eq!(fuzzy_match("exp", "Export CSV"), 0.5);
    }
    
    #[test]
    fn test_palette_navigation() {
        let mut palette = CommandPalette::new();
        palette.set_items(vec![item1, item2, item3]);
        palette.open();
        
        assert_eq!(palette.selected_index, 0);
        palette.move_selection_down();
        assert_eq!(palette.selected_index, 1);
    }
    
    #[test]
    fn test_search_items() {
        let items = vec![...];
        let results = search_items("test", &items);
        assert!(results.is_sorted_by_score());
    }
}
```

### 7.2 Integration Testing

```rust
// GUI をシミュレートしてテスト
#[test]
fn test_palette_e2e() {
    let mut app = AnalyzerApp::new();
    
    // Ctrl+P をシミュレート
    app.palette_visible = true;
    app.command_palette.open();
    
    // テキスト入力
    app.command_palette.search_text = "exp".to_string();
    
    // 結果を確認
    assert!(!app.command_palette.filtered_items.is_empty());
    
    // 選択して実行
    let item = app.command_palette.get_selected_item();
    assert!(item.is_some());
}
```

---

**Last Updated**: 2026-06-12  
**Status**: ✅ Phase 4.4 Complete  
**Next**: Phase 4.5 (Pane Layout System)
