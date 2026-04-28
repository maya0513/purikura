# Cloudflare デプロイ手順

このリポジトリは **Cloudflare Workers + Static Assets**（2024 年に Pages と統合された新方式）でデプロイします。GitHub 連携での自動デプロイを前提にした設定です。

## ファイル構成

| ファイル             | 役割                                                                                  |
| -------------------- | ------------------------------------------------------------------------------------- |
| `wrangler.jsonc`     | Workers 設定。`./dist` を静的サイトとして配信、`not_found_handling` で SPA fallback。 |
| `public/_headers`    | WASM の MIME、長期キャッシュ、セキュリティヘッダ                                      |
| `deploy/cf-build.sh` | Rust + wasm-pack インストール + ビルド                                                |

> SPA fallback は `wrangler.jsonc` の `assets.not_found_handling: "single-page-application"` で完結するので `_redirects` は置いていません。

## ダッシュボード設定（GitHub 連携時）

Cloudflare ダッシュボード → Workers & Pages → **Create** → **Connect to Git** で以下を入力:

| 項目                   | 値                         |
| ---------------------- | -------------------------- |
| Repository             | (このリポジトリ)           |
| Production branch      | `main`（ご自由に）         |
| Build command          | `bash deploy/cf-build.sh`  |
| Build output directory | `dist`                     |
| Root directory         | （空欄でリポジトリルート） |
| Environment variables  | （特になし）               |

`wrangler.jsonc` がリポジトリにあるので、Cloudflare は **Workers (with Static Assets)** として扱います（Pages ではなく）。

## ローカルでのプレビュー

```bash
# 通常通りビルド
just build

# Cloudflare Workers ランタイムをローカル起動（アカウント不要）
pnpm dlx wrangler@latest dev
```

`wrangler dev` は `dist/` を Workers Static Assets として配信し、`_headers` ルール
と SPA fallback を本番と同じ挙動で適用します。本番反映前の最終確認に使えます。

検証済み（手元での起動結果）:

- `/` → `index.html` 200、`Permissions-Policy`/`X-Frame-Options` ほか適用
- `/assets/*.wasm` → `Content-Type: application/wasm`、`Cache-Control: immutable`
- `/some/random/path` → SPA fallback で `index.html` 200
- WASM バイナリのマジックバイト `\0asm` 健全

## ビルドスクリプトの中身

`deploy/cf-build.sh` は以下を順に実行:

1. **Rust toolchain インストール** — CF のビルダ環境には Rust が無いため `rustup` を取得
2. **wasm-pack インストール** — 同上、`wasm-pack` 公式インストーラ
3. **`pnpm install --ignore-scripts`** — `prepare: vp config` は vp 不在の環境では失敗するのでスキップ
4. **`wasm-pack build`** — `src/wasm/pkg` に WASM 出力
5. **`pnpm exec vp build`** — vite-plus 同梱の `vp` バイナリで `dist/` を生成（`just build` と同じ）

CI 上での所要時間目安: Rust 初回 30-60 秒、それ以降 90 秒程度。

## カメラと HTTPS

`getUserMedia` は HTTPS が必須です。Cloudflare のデフォルトドメイン（`*.workers.dev`）は自動で HTTPS なので、追加設定不要です。

## セキュリティヘッダのメモ

- `_headers` で `Permissions-Policy: camera=(self)` を設定 → 自オリジンのみカメラ許可
- `Cross-Origin-Embedder-Policy` は本番では **設定していません**。`require-corp` だと MediaPipe のモデル（tfhub.dev 経由）が CORP 不在で読めなくなるため
- これにより SharedArrayBuffer は本番では使えませんが、TFJS WebGL バックエンドは SAB 不要なので問題なし

## トラブルシューティング

- **`vp` が無いと言われる** → `cf-build.sh` は `pnpm exec vite build` を直叩きするので `vp` は不要です
- **WASM の MIME が違う** → `_headers` の `Content-Type: application/wasm` 設定を確認。Cloudflare の `_headers` は `dist/` 配下のファイルにのみ適用される（`public/_headers` をビルド時に Vite が `dist/_headers` にコピーします）
- **MediaPipe の初回ロードが遅い** → tfhub.dev からのモデルダウンロード。初回 5-10 秒は仕様
