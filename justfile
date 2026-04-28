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

wasm-build:
    cd wasm && ../node_modules/.bin/wasm-pack build --target web --out-dir ../src/wasm/pkg --out-name purikura_wasm

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
