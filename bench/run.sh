#!/usr/bin/env bash
# Benchmarks the renderer on the screens in examples/gauges.json and
# turboxs.json, rasterising on the CPU. Needs a Rust toolchain.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release -q
for i in 1 2 3; do
	echo "=== run $i"
	for config in examples/gauges.json examples/turboxs.json; do
		target/release/gauge bench "$config" --frames 600 --warmup 300
	done
done
