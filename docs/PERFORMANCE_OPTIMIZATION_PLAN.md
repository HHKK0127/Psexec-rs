# Phase 4.6 実装プラン: Performance Optimization

**プラン名**: Phase 4.6 Performance Optimization  
**作成日**: 2026-06-15  
**期間**: 1 週間（実装時間: 9-10 日相当）  
**優先度**: 🔴 HIGH

---

## 📋 プランの目的

Pane Layout System（Phase 4.5）のパフォーマンスを Rio ターミナルスタイルの最適化レベルに引き上げ、以下を実現する：

- ✅ FPS 60 の安定化
- ✅ 描画時間の短縮（10-20ms → 5-10ms）
- ✅ メモリアロケーション削減（10%+）
- ✅ GC 回数の削減

---

## 🔍 現在の状況

### Phase 4.5 の実装状況
```
✅ LayoutNode: Binary tree 構造
✅ Renderer: egui ベース描画
✅ Events: divider ドラッグイベント
✅ Config: JSON 永続化
```

### 予想される性能ボトルネック
1. **毎フレーム全レイアウト再計算** - 変更なしでも計算
2. **メモリ確保** - 毎フレーム Vec/String 新規作成
3. **不要な再描画** - 変更なしでも全パネル再描画
4. **メトリクス未計測** - 最適化の効果測定ができない

---

## 🎯 実装アプローチ

### アーキテクチャ設計

#### 1. Damage Tracking System
**責務**: フレーム間の変更検出と再描画範囲限定

**ファイル**: `src/ui/pane_layout/damage.rs` (150+ 行)

```rust
// 変更検出結果
#[derive(Debug, Clone)]
pub struct DamageRegion {
    pub rects: Vec<Rect>,           // 変更された矩形
    pub full_redraw_needed: bool,   // 全描画が必要か
}

// 状態を保持して変更検出
pub struct DamageTracker {
    previous_state: Option<PaneLayoutState>,
    previous_rects: HashMap<NodeId, Rect>,
}

impl DamageTracker {
    /// 新規作成
    pub fn new() -> Self { ... }
    
    /// 現在の状態と前フレーム状態を比較
    pub fn track_changes(
        &mut self,
        current_state: &PaneLayoutState,
    ) -> DamageRegion {
        // 1. active_pane の変更を検出
        // 2. resizing_divider の状態変更を検出
        // 3. divider_pos の変更を検出
        // 4. 変更されたノードの矩形を計算
        // 5. 状態を保存
    }
    
    /// 指定矩形が再描画対象か判定
    pub fn should_redraw(&self, rect: &Rect) -> bool { ... }
}

// 使用方法:
// let damage = tracker.track_changes(state);
// if damage.full_redraw_needed {
//     redraw_all();
// } else {
//     for rect in damage.rects {
//         redraw_region(rect);
//     }
// }
```

**実装詳細**:
- `PaneLayoutState` のクローンを保持（メモリコスト < 1KB）
- divider_pos 変更時は分割ノード周辺のみダメージ
- active_pane 変更時はヘッダー再描画のみ

**テスト項目**:
- [ ] 静止フレーム：ダメージなし
- [ ] divider ドラッグ中：divider 周辺のみ
- [ ] パネル切り替え：ヘッダーのみ
- [ ] 複数変更：全変更を検出

---

#### 2. Memory Pool System
**責務**: アロケーション削減とメモリ再利用

**ファイル**: `src/ui/pane_layout/memory_pool.rs` (200+ 行)

```rust
// バッファプール
pub struct MemoryPool {
    // Vec<u8> プール（テンポラリバッファ用）
    vec_u8_pool: Vec<Vec<u8>>,
    
    // String プール（テキスト生成用）
    string_pool: Vec<String>,
    
    // Rect プール（レイアウト計算用）
    rect_pool: Vec<Rect>,
    
    // 統計情報
    stats: PoolStats,
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub allocations: usize,      // 新規アロケーション数
    pub reuses: usize,           // 再利用数
    pub total_bytes: usize,      // 総容量
}

impl MemoryPool {
    /// 新規作成（初期容量指定）
    pub fn new(capacity: usize) -> Self { ... }
    
    /// Vec<u8> を取得（再利用または新規作成）
    pub fn get_vec_u8(&mut self, min_capacity: usize) -> Vec<u8> {
        if let Some(mut vec) = self.vec_u8_pool.pop() {
            if vec.capacity() >= min_capacity {
                vec.clear();
                self.stats.reuses += 1;
                return vec;
            }
        }
        self.stats.allocations += 1;
        Vec::with_capacity(min_capacity)
    }
    
    /// Vec<u8> を返却（プールに戻す）
    pub fn return_vec_u8(&mut self, mut vec: Vec<u8>) {
        vec.clear();
        if self.vec_u8_pool.len() < 16 {  // 最大16個まで保持
            self.vec_u8_pool.push(vec);
        }
    }
    
    /// String を取得（再利用または新規作成）
    pub fn get_string(&mut self) -> String {
        if let Some(mut s) = self.string_pool.pop() {
            s.clear();
            self.stats.reuses += 1;
            return s;
        }
        self.stats.allocations += 1;
        String::new()
    }
    
    /// フレーム終了時にリセット
    pub fn reset_frame(&mut self) {
        // 使用済みバッファを保持、次フレームで再利用
        self.stats.allocations = 0;
        self.stats.reuses = 0;
    }
    
    /// 統計情報取得
    pub fn stats(&self) -> &PoolStats { &self.stats }
}

// 使用方法:
// let mut vec = pool.get_vec_u8(1024);
// // ... 使用 ...
// pool.return_vec_u8(vec);
```

**実装詳細**:
- Vec サイズは自動増加（Rust のデフォルト動作）
- String の再利用で str アロケーション削減
- フレームごとにリセットして新鮮な状態を保証

**テスト項目**:
- [ ] 取得/返却サイクル
- [ ] メモリ再利用確認（stats.reuses > 0）
- [ ] アロケーション削減（10%+）
- [ ] メモリリーク検出

---

#### 3. パフォーマンス計測システム
**責務**: FPS、描画時間、メモリ使用量の計測

**ファイル**: `src/ui/pane_layout/perf_metrics.rs` (150+ 行)

```rust
use std::collections::VecDeque;

// パフォーマンスメトリクス
pub struct PerfMetrics {
    // 環形バッファ（最新100フレーム）
    frame_times: VecDeque<f32>,           // ms
    render_times: VecDeque<f32>,          // ms
    memory_usage: VecDeque<usize>,        // bytes
    
    // 統計情報
    last_update: std::time::Instant,
}

impl PerfMetrics {
    /// 新規作成
    pub fn new() -> Self {
        Self {
            frame_times: VecDeque::with_capacity(100),
            render_times: VecDeque::with_capacity(100),
            memory_usage: VecDeque::with_capacity(100),
            last_update: std::time::Instant::now(),
        }
    }
    
    /// フレーム時間を記録（ミリ秒）
    pub fn record_frame(&mut self, duration_ms: f32) {
        if self.frame_times.len() >= 100 {
            self.frame_times.pop_front();
        }
        self.frame_times.push_back(duration_ms);
    }
    
    /// 描画時間を記録（ミリ秒）
    pub fn record_render(&mut self, duration_ms: f32) {
        if self.render_times.len() >= 100 {
            self.render_times.pop_front();
        }
        self.render_times.push_back(duration_ms);
    }
    
    /// メモリ使用量を記録（バイト）
    pub fn record_memory(&mut self, bytes: usize) {
        if self.memory_usage.len() >= 100 {
            self.memory_usage.pop_front();
        }
        self.memory_usage.push_back(bytes);
    }
    
    // === 統計計算 ===
    
    /// 平均 FPS
    pub fn average_fps(&self) -> f32 {
        if self.frame_times.is_empty() { return 0.0; }
        let avg_ms: f32 = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
        1000.0 / avg_ms
    }
    
    /// 最低 FPS（最もフレーム時間が長かった）
    pub fn min_fps(&self) -> f32 {
        if let Some(&max_ms) = self.frame_times.iter().max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)) {
            1000.0 / max_ms
        } else {
            0.0
        }
    }
    
    /// 平均描画時間（ミリ秒）
    pub fn average_render_ms(&self) -> f32 {
        if self.render_times.is_empty() { return 0.0; }
        self.render_times.iter().sum::<f32>() / self.render_times.len() as f32
    }
    
    /// P95 描画時間（ミリ秒）
    pub fn render_time_p95_ms(&self) -> f32 {
        if self.render_times.is_empty() { return 0.0; }
        let mut sorted: Vec<f32> = self.render_times.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx = (sorted.len() as f32 * 0.95) as usize;
        sorted[idx.min(sorted.len() - 1)]
    }
    
    /// ピークメモリ使用量（MB）
    pub fn peak_memory_mb(&self) -> f32 {
        if let Some(&max_bytes) = self.memory_usage.iter().max() {
            max_bytes as f32 / 1_000_000.0
        } else {
            0.0
        }
    }
    
    /// 平均メモリ使用量（MB）
    pub fn average_memory_mb(&self) -> f32 {
        if self.memory_usage.is_empty() { return 0.0; }
        let avg_bytes: usize = self.memory_usage.iter().sum::<usize>() / self.memory_usage.len();
        avg_bytes as f32 / 1_000_000.0
    }
}

// 使用方法:
// let frame_start = std::time::Instant::now();
// render(...);
// metrics.record_frame(frame_start.elapsed().as_secs_f32() * 1000.0);
// println!("FPS: {}", metrics.average_fps());
```

**実装詳細**:
- 環形バッファで最新100フレーム保持
- P95, P99 などの分位数計算対応
- メモリ効率: ~5KB（100フレーム分）

**テスト項目**:
- [ ] フレーム時間記録
- [ ] FPS 計算精度（1 フレーム 16.67ms → 60 FPS）
- [ ] P95 計算確認
- [ ] メモリ使用量追跡

---

#### 4. 統合レイアウト
**ファイル**: `src/ui/pane_layout/renderer.rs` (修正)

```rust
pub struct OptimizedPaneRenderer {
    damage_tracker: DamageTracker,
    memory_pool: MemoryPool,
    perf_metrics: PerfMetrics,
}

impl OptimizedPaneRenderer {
    pub fn new() -> Self {
        Self {
            damage_tracker: DamageTracker::new(),
            memory_pool: MemoryPool::new(1024 * 100),  // 100KB 初期
            perf_metrics: PerfMetrics::new(),
        }
    }
    
    pub fn render_pane_layout(
        &mut self,
        ctx: &egui::Context,
        state: &mut PaneLayoutState,
        available_rect: Rect,
    ) {
        let frame_start = std::time::Instant::now();
        
        // 1. 変更検出
        let damage = self.damage_tracker.track_changes(state);
        
        // 2. 最適化描画
        if damage.full_redraw_needed {
            render_full(ctx, state, available_rect);
        } else {
            render_damaged_regions(ctx, state, &damage.rects);
        }
        
        // 3. フレーム終了処理
        self.memory_pool.reset_frame();
        self.perf_metrics.record_frame(
            frame_start.elapsed().as_secs_f32() * 1000.0
        );
        
        // 4. デバッグ UI（Ctrl+D で表示）
        if ctx.input(|i| i.key_pressed(egui::Key::D) && i.modifiers.ctrl) {
            render_perf_overlay(ctx, &self.perf_metrics);
        }
    }
}

fn render_perf_overlay(ctx: &egui::Context, metrics: &PerfMetrics) {
    egui::Window::new("Performance Metrics")
        .open(&mut true)
        .show(ctx, |ui| {
            ui.label(format!("FPS: {:.1}", metrics.average_fps()));
            ui.label(format!("Min FPS: {:.1}", metrics.min_fps()));
            ui.label(format!("Render: {:.2}ms avg, {:.2}ms p95", 
                metrics.average_render_ms(),
                metrics.render_time_p95_ms()));
            ui.label(format!("Memory: {:.1}MB avg, {:.1}MB peak",
                metrics.average_memory_mb(),
                metrics.peak_memory_mb()));
        });
}
```

---

## 📊 実装ステップ

### ステップ 1: Damage Tracking (2-3 日)

**実装内容**:
1. `src/ui/pane_layout/damage.rs` 作成
2. `DamageTracker::track_changes()` 実装
3. 変更検出ロジック（divider_pos, active_pane, hover_divider）
4. ユニットテスト

**コミット**: `feat: Add Damage Tracking System for incremental rendering`

**テスト**:
```bash
cargo test damage_tracker
```

**確認項目**:
- [ ] 静止フレーム：damage.rects.is_empty()
- [ ] divider ドラッグ：divider 周辺のみ
- [ ] hover 状態：divider ヘッダー領域のみ

---

### ステップ 2: Memory Pool (1.5-2 日)

**実装内容**:
1. `src/ui/pane_layout/memory_pool.rs` 作成
2. `MemoryPool` の取得/返却実装
3. 統計情報の記録
4. 統合テスト

**コミット**: `feat: Add Memory Pool for allocation reduction`

**テスト**:
```bash
cargo test memory_pool
```

**確認項目**:
- [ ] 再利用率: stats.reuses > 80%
- [ ] アロケーション削減: 新規作成 < 前フレーム10%

---

### ステップ 3: パフォーマンス計測 (1-1.5 日)

**実装内容**:
1. `src/ui/pane_layout/perf_metrics.rs` 作成
2. 統計計算メソッド実装
3. デバッグ UI (Ctrl+D)
4. ログ出力

**コミット**: `feat: Add Performance Metrics System for monitoring`

**テスト**:
```bash
cargo test perf_metrics
```

**確認項目**:
- [ ] FPS 計算: 1ms フレーム → 1000 FPS
- [ ] P95 計算: ソート順序確認
- [ ] メモリ計測: RSS との照合

---

### ステップ 4: 統合とチューニング (2-2.5 日)

**実装内容**:
1. `src/ui/pane_layout/mod.rs` に新モジュール追加
2. `renderer.rs` を `OptimizedPaneRenderer` に切り替え
3. `src/ui/app.rs` に統合
4. エンドツーエンドテスト
5. パフォーマンス測定と最適化

**コミット**: 
- `feat: Integrate Damage Tracking and Memory Pool into renderer`
- `perf: Optimize divider dragging performance`

**テスト項目**:
- [ ] コンパイル成功
- [ ] GUI 起動確認
- [ ] divider ドラッグ滑らか（60 FPS）
- [ ] メモリ使用量（< 50MB）
- [ ] デバッグ UI 表示確認

---

### ステップ 5: ドキュメント更新 (0.5-1 日)

**実装内容**:
1. `docs/ARCHITECTURE.md` に Phase 4.6 セクション追加
2. `docs/PANE_LAYOUT_IMPLEMENTATION.md` パフォーマンス情報追加
3. `docs/ROADMAP.md` チェックリスト更新
4. このプランファイルを完了マークで更新

**コミット**: `docs: Add Phase 4.6 Performance Optimization documentation`

---

## 🎯 実装パラメータ

### Damage Tracker
```rust
const DAMAGE_HISTORY_SIZE: usize = 100;  // 最新100フレーム保持
const MIN_DAMAGE_RECT_SIZE: f32 = 4.0;   // 4px 以下は無視
```

### Memory Pool
```rust
const VEC_U8_POOL_CAPACITY: usize = 16;   // 最大16個
const STRING_POOL_CAPACITY: usize = 32;   // 最大32個
const RECT_POOL_CAPACITY: usize = 256;    // 最大256個
```

### パフォーマンス目標
```rust
const TARGET_FPS: f32 = 60.0;
const MAX_FRAME_TIME_MS: f32 = 16.67;     // 60 FPS
const MAX_RENDER_TIME_MS: f32 = 10.0;     // 描画上限
const TARGET_MEMORY_MB: f32 = 50.0;       // メモリ上限
```

---

## 📈 期待される改善

| メトリクス | Phase 4.5 | Phase 4.6 | 改善率 |
|----------|-----------|-----------|-------|
| **平均 FPS** | 50-58 | 60 | +10% |
| **最低 FPS** | 30-40 | 55+ | +50% |
| **描画時間平均** | 10-20ms | 5-10ms | 50% 削減 |
| **P95 描画時間** | 20-30ms | 10-15ms | 50% 削減 |
| **アロケーション** | 毎フレーム多数 | 最小限 | 80% 削減 |
| **メモリ（RSS）** | 55-60MB | 45-50MB | 10% 削減 |

---

## ✅ テスト戦略

### ユニットテスト
```bash
cargo test damage_tracker
cargo test memory_pool
cargo test perf_metrics
```

### 統合テスト
```bash
cargo test --test integration_tests -- pane_layout
```

### パフォーマンステスト
1. GUI 起動
2. divider ドラッグ 30 秒間
3. Ctrl+D でメトリクス表示
4. 目標値確認

**判定基準**:
- ✅ FPS ≥ 60
- ✅ P95 ≤ 15ms
- ✅ メモリ ≤ 50MB

---

## 🔗 依存関係

| 項目 | 状態 | 備考 |
|------|------|------|
| Phase 4.5 | ✅ 完了 | 基盤実装完了 |
| egui 0.27 | ✅ 利用可能 | 既存 |
| std::time | ✅ 利用可能 | 計測用 |

---

## 📚 関連ドキュメント

- [ARCHITECTURE.md](./ARCHITECTURE.md) - GUI アーキテクチャ
- [PANE_LAYOUT_IMPLEMENTATION.md](./PANE_LAYOUT_IMPLEMENTATION.md) - Phase 4.5 詳細
- [ROADMAP.md](./ROADMAP.md) - マルチフェーズロードマップ

---

## 🚀 実装開始の確認

**このプランで実装を開始してよろしいですか？**

実装予定:
1. ✅ 詳細な実装粒度（モジュール・関数単位）
2. ✅ テスト範囲（ユニット・統合・パフォーマンス）
3. ✅ デバッグ UI（Ctrl+D でメトリクス表示）
4. ✅ パフォーマンス目標値（FPS 60, 描画 5-10ms）

---

**Status**: 📋 プラン作成完了  
**Ready**: 🚀 実装開始待機中  
**Estimated Duration**: 1 週間

