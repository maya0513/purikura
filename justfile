default:
    @just --list

setup:
    rustup target add wasm32-unknown-unknown
    vp install

dev: wasm-build
    vp dev

build: wasm-build
    vp build

test: wasm-test frontend-test

coverage:
    vp test run --coverage

wasm-build:
    cd wasm && ../node_modules/.bin/wasm-pack build --target web --out-dir ../src/wasm/pkg --out-name purikura_wasm

wasm-build-node:
    cd wasm && ../node_modules/.bin/wasm-pack build --target nodejs --out-dir ../src/wasm/pkg_node --out-name purikura_wasm

deploy: build
    node_modules/.bin/wrangler deploy

samples: wasm-build
    node_modules/.bin/tsx scripts/generate_samples.ts

wasm-test:
    cd wasm && cargo test

frontend-test:
    vp test run

check:
    vp check
    cd wasm && cargo clippy -- -D warnings

lint:
    vp lint src/

fmt:
    vp fmt src/
    cd wasm && cargo fmt

clean:
    rm -rf dist src/wasm/pkg
    cd wasm && cargo clean

preview: build
    vp preview
