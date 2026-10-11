#!/usr/bin/env bash
# Benchmarks the C# (SkiaSharp) and Rust (tiny-skia) renderers on the same
# screens (rust/examples/gauges.json), both rasterising on the CPU.
# Needs the .NET 10 SDK and a Rust toolchain.
set -euo pipefail
cd "$(dirname "$0")/.."
dotnet build -c Release bench/csharp/GaugeBench.csproj -v q -nologo >/dev/null
cargo build --release --manifest-path rust/Cargo.toml -q
for i in 1 2 3; do
	echo "=== run $i"
	bench/csharp/bin/Release/net10.0/GaugeBench rust/examples/gauges.json 600 300
	rust/target/release/gauge bench rust/examples/gauges.json --frames 600 --warmup 300
done
