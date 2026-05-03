# purikura

プリクラ風 Web アプリ。カメラで1枚撮影 → 顔加工・装飾を編集 → 画像としてダウンロード。

## 技術スタック

- **UI**: Preact + TypeScript + `@preact/signals`
- **画像処理**: Rust → WASM (`wasm-pack`)。`wasm/` ディレクトリ。
- **顔ランドマーク**: `@tensorflow-models/face-landmarks-detection`（WebGL backend）
- **スタイル**: Tailwind CSS v4
- **ツールチェーン**: Vite+ (`vp` CLI)
- **タスクランナー**: `just`

## アプリのフロー

`AppState`: `idle | countdown | capturing | edit`

1. `idle` — トップ画面、`canStartCapture`（`wasmReady` が前提）で撮影開始
2. `countdown` — カウントダウン後に
3. `capturing` — Camera コンポーネントが1枚キャプチャ → `capturedPhoto` (data URL) にセット
4. `edit` — 編集パネルで以下を調整しながらリアルタイムプレビュー、確定でダウンロード

写真サイズは固定 `PHOTO_WIDTH=640 × PHOTO_HEIGHT=480`（`src/state/types.ts`）。

## 編集レイヤ（適用順）

1. **美容加工** (`BeautyPanel`, WASM) — `BeautyParams` の各 strength で制御
   - `skin`: 美肌（guided filter ベースのスムージング + プリクラ調トーン）
   - `blemish`: シミ・赤み除去
   - `eyes`: 虹彩中心の目拡大ワープ
2. **フィルター** (`FilterPanel`, WASM) — `none | grayscale | sepia | vivid | soft | warm | cool`
3. **フレーム** (`FramePanel`, WASM 合成) — `none | hearts | stars | flowers | bubbles`
4. **スタンプ** (`StickerPanel`) — `DekoItem[]`（絵文字を `(x, y)` に配置）

状態は `src/state/signals.ts` の signal で持つ。`processedUrl` が最終プレビュー。

## WASM API（`wasm/src/lib.rs`）

JS から呼ぶ前提のコントラクト。座標・寸法は基本的に画像の正規化値（0..1）。

- `apply_filter(pixels, w, h, filter_name) -> Vec<u8>`
- `compose_frame(photo, frame, w, h) -> Vec<u8>` — alpha 合成
- `build_skin_mask(pixels, w, h, face_oval, exclusions_packed) -> Vec<u8>`
  - `face_oval`: 顔輪郭ポリゴン `[x0,y0,x1,y1,...]`（0..1 正規化）
  - `exclusions_packed`: 目・口など除外ポリゴンのパック形式 `[n_polys, len_0, x,y,..., len_1, x,y,...]`
  - フェザー半径は `MASK_FEATHER_RADIUS = 4`
- `apply_beauty(pixels, w, h, mask, strength) -> Vec<u8>`
- `remove_blemish(pixels, w, h, mask, strength) -> Vec<u8>`
- `enlarge_eyes(pixels, w, h, eyes, strength) -> Vec<u8>`
  - `eyes`: `[cx0, cy0, r0, cx1, cy1, r1, ...]`（cx は width 正規化、cy は height 正規化、典型的に虹彩半径 × 2.5）

**重要**: `mask` は呼び出し側で `build_skin_mask` を1回計算し、`apply_beauty` / `remove_blemish` で使い回す（毎回再計算しない）。

## ディレクトリ

```
src/
  app.tsx            アプリ本体（状態遷移と編集パネルの結線）
  components/        Camera / BeautyPanel / FilterPanel / FramePanel / StickerPanel / PhotoStrip
  hooks/             useAppState / useCamera / useCountdown / useFaceFrame / useWasm
  lib/               imageProcessor / faceLandmarks / frameRenderer / countdown / stateMachine
  state/             signals.ts, types.ts
  wasm/pkg/          wasm-pack 出力（gitignore）
wasm/src/            Rust 実装（beauty / blemish / compositor / eye_warp / filters / guided_filter / skin_mask）
```

## 開発コマンド（`just`）

- `just setup` — Rust target 追加 + `vp install`
- `just dev` — `wasm-build` 後に dev server
- `just build` — `wasm-build` 後にプロダクションビルド
- `just wasm-build` — Rust → WASM（**Rust 側を変更したら必須**。`src/wasm/pkg/` に出力）
- `just test` — Rust テスト（`cargo test`）+ フロントエンドテスト（`vp test run`）
- `just check` — `vp check` + `cargo clippy -- -D warnings`
- `just fmt` / `just lint` / `just clean`

## 注意点

- `vite.config.ts` は `import { defineConfig } from 'vite-plus'`（`vite` から import しない）
- テストユーティリティは `vite-plus/test`（`vitest` から import しない）
- `~/` パスエイリアスは `src/` を指す
- `pnpm.onlyBuiltDependencies` に `wasm-pack`, `@swc/core` が必要
- パスエイリアスや WASM ロードに関わるので、依存追加時は `vite.config.ts` も確認
