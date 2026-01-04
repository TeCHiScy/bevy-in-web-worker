.PHONY: run build

# cargo install basic-http-server
# worker 中运行时，debug 模式下会有前几帧须要拉长帧时间间隔的问题
# https://github.com/bevyengine/bevy/issues/13345
run:
	cargo build --no-default-features --profile dev-opt --target wasm32-unknown-unknown
	wasm-bindgen --no-typescript --out-dir public --out-name bevy --web target/wasm32-unknown-unknown/dev-opt/*.wasm
	basic-http-server public

# 优化 wasm 包大小
# https://github.com/WebAssembly/binaryen/releases
build:
	cargo build --no-default-features --profile wasm-release --target wasm32-unknown-unknown 
	wasm-bindgen --no-typescript --out-dir public --out-name bevy --web target/wasm32-unknown-unknown/wasm-release/*.wasm
	wasm-opt -Oz --output public/bevy_bg.wasm public/bevy_bg.wasm