#!/usr/bin/env bash
# Benchmarks the renderer on the screens in rust/examples/gauges.json and
# turboxs.json, rasterising on the CPU. Needs a Rust toolchain.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release --manifest-path rust/Cargo.toml -q
for i in 1 2 3; do
	echo "=== run $i"
	for config in rust/examples/gauges.json rust/examples/turboxs.json; do
		rust/target/release/gauge bench "$config" --frames 600 --warmup 300
	done
done
