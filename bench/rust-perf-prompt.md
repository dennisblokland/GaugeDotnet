# Prompt: speed up the slowest Rust gauges on the RG35XX

Paste this into a new Claude Code session in `/home/dennis/GaugeDotnet`.

---

The Rust renderer (`rust/gauge-render`, tiny-skia 0.11) is 3–15× faster than the C# app on a desktop, but on the RG35XX it loses most of that lead, and on the Histogram gauge it's slower than C#. Find out why and make the slowest screens faster on the device, without changing what they draw.

## Measured on the device (ms/frame)

The device is an RG35XX running muOS: 4× Cortex-A53 at up to 1512 MHz, a Mali-G31 GPU, glibc 2.38, reachable as `root@192.168.0.234` with SSH key auth. Each figure is 600 timed frames after 300 warm-up frames, with new data every frame. Rust runs with 4 threads, but the built-in gauges are single-threaded.

| Screen | Rust | C#, CPU raster | C#, GPU (GLES2, GPU clock pinned) |
|---|---:|---:|---:|
| Histogram (Clt) | **37.0** | 25.9 | 18.6 |
| Circular (Rpm) | **16.1** (22.5 in another run) | 21.8 | 22.1 |
| Sweep (Map) | **11.7** | 22.9 | 13.2 |
| MinMax (OilTemp) | 8.7 | 19.4 | 16.5 |
| Custom (showcase.json) | 6.5 | 18.9 | 12.4 |
| Digital | 5.5 | 12.2 | 9.2 |
| Needle | 5.1 | 13.6 | 15.6 |
| Bar | 4.8 | 21.7 | 13.5 |
| TurboXs, `"Lcd": true` | 4.7 | 18.2 | 3.7 |
| TurboXs | 4.4 | 6.7 | 6.5 |
| Grid | 3.0 | 6.3 | 7.6 |

The app targets 60 fps (16.7 ms per frame). Histogram (27 fps) misses it, Circular is borderline, and Sweep leaves little headroom.

## The key clue

From a desktop x86 (Ryzen 7 3800X) to the device:
- every C# screen gets **3.5–4.5×** slower;
- every Rust screen gets **8.4–11×** slower.

Histogram isn't an outlier in that ratio (4.0 → 37.0 ms). It's simply the heaviest screen. So the main suspect is something that makes all Rust rendering lose about 2.5× on aarch64 compared with x86, rather than a single bad algorithm. Hypotheses to confirm or rule out, in this order:

1. **SIMD on aarch64.** Skia's CPU pipelines are NEON-tuned. Check whether tiny-skia 0.11's raster pipeline (lowp/highp), and our own hot loops (`gauge-render/src/blur.rs`, `composite.rs`, glow upscaling, sprite blending), get vectorised for `aarch64-unknown-linux-gnu`, or fall back to scalar code that is fast on x86 only thanks to wide out-of-order cores. Look at tiny-skia's cfg-gated SIMD code, and inspect the release assembly of our hot loops (`cargo asm`, or `objdump` of the aarch64 build) for NEON instructions.
2. **Codegen flags.** The device build has no `-C target-cpu=cortex-a53`. Try it, and also check `lto`, `codegen-units = 1` and `opt-level` in the workspace `Cargo.toml` release profile.
3. **Memory bandwidth and cache.** The A53 has a small L2 and slow DRAM. Full-frame 640×480 RGBA passes (clears, glow composites, layer copies) cost proportionally more there. Count how many full-frame passes each slow gauge makes per frame.
4. **Per-gauge work:** Histogram (bars + glows, `gauges/histogram.rs`), Circular (`gauges/circular.rs`), Sweep (glowing arc, not cached, `gauges/sweep.rs`).

## How to measure

- **Desktop:** `cd rust && cargo run --release -- bench examples/gauges.json --frames 600 --warmup 300`. Add `--threads 1` and `--full` for the worst case.
- **Device build:** `rust/build-arm64.sh`. It falls back to a `rust:1-bookworm` Docker container when `aarch64-linux-gnu-gcc` is missing, and puts the binary at `rust/target/aarch64-unknown-linux-gnu/release/gauge`. For bench-only builds, a headless build without `sdl,ble-vendored` is enough.
- **Device bench:** copy the binary plus `rust/examples/*.json` to `/mnt/mmc/ports/GaugeBench/`. Run `./gauge bench examples/gauges.json --frames 600 --warmup 300`. During long runs, keep muOS from idling: run `echo 1 > /opt/muos/config/system/idle_inhibit` every second in a background loop. Otherwise the screen saver drops the CPU to 480 MHz mid-run. Log `scaling_cur_freq` before each run.
- **Profiling on the device:** check whether `perf` exists on the device. If not, use a statically built `perf`, or instrument with timers around each drawing stage. Comparing the same stages under `qemu-aarch64` won't give real timings; use it only to check correctness.
- **C# comparison** (`bench/csharp`): `GaugeBench <config> [frames] [warmup] [--gpu] [--png dir]`. A linux-arm64 self-contained publish is already on the device at `/mnt/mmc/ports/GaugeBench/csharp/`.

## Constraints

- Output must stay identical to what it is now. `cargo test` has render tests, and the aarch64 headless build was previously checked to match the x86 output pixel for pixel. Re-check that after any SIMD or codegen change, for example with `gauge render ... -o` on both machines and a diff of the PNGs.
- Don't change the device's clocks or governors from the bench. `GaugeDotnet.sh` pins the GPU clock for the C# app only.
- When done, update `bench/RESULTS.md` section 5 and the Performance section of `rust/README.md` with before and after numbers measured on the device. Also update README.md and AGENTS.md if behaviour or build flags change.

Report which hypothesis held, the device numbers before and after for every screen, and anything you tried that didn't help.
