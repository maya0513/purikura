#!/usr/bin/env bash
# Cloudflare build script for Workers (Static Assets) / Pages.
#
# CF's managed CI image ships Node + npm but no Rust toolchain. We install
# rustup + wasm-pack on the fly, then run the project's standard build
# (wasm-pack → vite). The local `vite` package is overridden to
# `@voidzero-dev/vite-plus-core` (see package.json), so `pnpm exec vite build`
# produces the same output as a local `vp build`.

set -euo pipefail

echo "::: build env"
node --version
echo "platform: $(uname -a || true)"

# --- Rust toolchain --------------------------------------------------------
if ! command -v cargo >/dev/null 2>&1; then
  echo "::: installing rust"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain stable --profile minimal --no-modify-path
fi
# shellcheck disable=SC1091
source "$HOME/.cargo/env"
rustup target add wasm32-unknown-unknown
rustc --version

# --- wasm-pack -------------------------------------------------------------
if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "::: installing wasm-pack"
  curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi
wasm-pack --version

# --- pnpm via corepack -----------------------------------------------------
corepack enable >/dev/null 2>&1 || true
# packageManager pin in package.json is honoured automatically
pnpm --version

# --- Install JS deps -------------------------------------------------------
# `--ignore-scripts` skips the project's `prepare: vp config` (vp is a global
# Vite+ binary not present on the CF builder; not needed for the build).
echo "::: installing dependencies"
pnpm install --frozen-lockfile --ignore-scripts

# --- Build WASM ------------------------------------------------------------
echo "::: building wasm"
(cd wasm && wasm-pack build --target web --out-dir ../src/wasm/pkg --out-name purikura_wasm)

# --- Build frontend --------------------------------------------------------
# Local `vite` is overridden to `@voidzero-dev/vite-plus-core`, equivalent to
# `vp build`.
echo "::: building frontend"
pnpm exec vite build

echo "::: done"
ls -la dist/
