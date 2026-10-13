# Full benchmark results

All runs: one session, Intel Xeon @ 2.10 GHz (4 vCPU cloud VM), Linux x64, .NET 10.0.112 / SkiaSharp 3.119, Rust 1.97. Setup:
- 640×480 CPU raster, simulated ECU data stepped at 60 Hz;
- 300 warm-up frames, then 600 timed frames, timing only the draw call;
- each figure is the median of 3 runs, with runs alternating between implementations.

Re-run with `bench/run.sh` and the `gauge bench` flags below.

## 1. C# vs Rust, full repaint every frame (like for like)

C# redraws everything each frame, so Rust is shown with `--full`.

| Screen | C# (min–max) | Rust, 1 thread (min–max) | Speedup | Rust, 4 threads |
|---|---:|---:|---:|---:|
| Custom (showcase.json) | 8.12 (7.72–8.29) | 2.22 (2.02–2.39) | 3.7× | 1.66 |
| Circular | 10.34 (10.17–10.57) | 2.06 (2.04–2.21) | 5.0× | 2.23* |
| Needle | 6.42 (6.41–6.62) | 0.74 (0.61–0.78) | 8.7× | 0.65* |
| Sweep | 8.05 (7.89–8.61) | 1.64 (1.52–1.73) | 4.9× | 1.57* |
| Bar | 8.80 (8.77–9.52) | 0.67 (0.65–0.76) | 13.1× | 0.57* |
| Digital | 5.99 (5.91–6.00) | 0.73 (0.69–0.83) | 8.2× | 0.71* |
| MinMax | 8.98 (8.14–9.02) | 1.20 (1.07–1.40) | 7.5× | 1.11* |
| Histogram | 10.46 (10.41–10.85) | 4.84 (4.69–5.21) | 2.2× | 4.59* |
| Grid | 3.18 (2.92–3.19) | 0.40 (0.38–0.41) | 8.0× | 0.39* |

\* Built-in gauges are single-threaded; these differ from the 1-thread column only by noise.

## 2. Rust in normal operation (damage tracking on), ms/frame

| Screen | New data every frame, 1 thread / 4 threads | Repainted | 20 Hz data, 1 thread / 4 threads | Repainted | Values settled, 1 thread / 4 threads | Repainted |
|---|---:|---:|---:|---:|---:|---:|
| Custom (showcase.json) | 1.36 / 1.04 | 26 % | 1.29 / 0.99 | 25 % | 0.075 / 0.076 | 3 % |
| Circular | 2.12 / 2.22 | 100 % | 1.77 / 1.99 | 100 % | 0.50 / 0.44 | 100 % |
| Needle | 0.67 / 0.67 | 100 % | 0.56 / 0.48 | 100 % | 0.50 / 0.54 | 100 % |
| Sweep | 1.45 / 1.62 | 100 % | 1.41 / 1.34 | 100 % | 1.17 / 1.26 | 100 % |
| Bar | 0.58 / 0.59 | 100 % | 0.55 / 0.49 | 100 % | 0.32 / 0.26 | 100 % |
| Digital | 0.72 / 0.66 | 100 % | 0.42 / 0.34 | 100 % | 0.21 / 0.18 | 100 % |
| MinMax | 1.10 / 1.05 | 100 % | 1.06 / 1.00 | 100 % | 0.52 / 0.49 | 100 % |
| Histogram | 4.44 / 5.03 | 100 % | 2.10 / 1.93 | 100 % | 0.26 / 0.22 | 100 % |
| Grid | 0.37 / 0.41 | 100 % | 0.39 / 0.39 | 100 % | 0.36 / 0.35 | 100 % |

Damage tracking and threads apply to custom gauges only. The built-in gauges always repaint the full frame, but they get faster as data arrives less often because their caches (lit segments, digits, Histogram bars) hit more. Sweep stays around 1.2–1.5 ms because its glowing arc isn't cached.

## 3. TurboXs (`rust/examples/turboxs.json`)

Measured in a later session on a different VM (Intel Xeon @ 2.80 GHz, 4 vCPU, .NET 10.0.12, Rust 1.97), so compare these rows with each other, not with sections 1–2. New data every frame, full repaint, Rust on 1 thread; each figure is the median of 3 runs.

| Screen | C# (min–max) | Rust (min–max) | Speedup |
|---|---:|---:|---:|
| TurboXs | 3.27 (3.24–3.68) | 1.08 (1.04–1.14) | 3.0× |
| TurboXs, `"Lcd": true` | 5.18 (5.11–5.67) | 1.47 (1.37–1.65) | 3.5× |

- **Smaller Rust lead than on other gauges:** TurboXs has no blurred glows, which is where the Rust renderer's biggest shortcuts are. Its per-frame work is mostly glyph outlines (needle, readout, peak text), which both libraries draw at similar cost.
- **LCD mode:** the gauge renders at a third of the resolution (about 0.4 ms in Rust and 0.6 ms in C#), then one pass fills the 640×480 frame using lookup tables and row copies.
  - **CPU canvas:** C# takes this path too, because Skia's CPU path is slow at repeating-tile shaders. Most of C#'s remaining LCD cost is its full-screen `DrawBitmap`, about 2.5 ms.
  - **Device (GPU):** C# uses a single shader draw instead, which this benchmark can't measure.
  - **Before the lookup-table path:** LCD mode was 15.9 ms (C#) and 11.1 ms (Rust).

## 4. Resources

| | C# | Rust |
|---|---:|---:|
| Peak RSS, whole benchmark | 81 MB | 24 MB |
| Deployable size | .NET runtime + app + 11 MB `libSkiaSharp.so` | 2.6 MB stripped binary (fonts embedded) |

## 5. On the device (RG35XX)

One run on the device itself: 4× Cortex-A53 at up to 1.5 GHz (`ondemand` governor), Mali-G31 GPU, muOS, .NET 10.0.12, Rust 1.99 (built in a `rust:1-bookworm` container). 300 warm-up frames, then 600 timed frames, new data every frame. C# is shown twice:
- on a CPU raster surface, as in sections 1–3;
- with `--gpu`, which is how the app runs: Skia's GLES2 backend through SDL's `mali` driver, flushed and waited on every frame.

Rust runs as in normal operation (damage tracking on, 4 threads), so only Custom repaints less than the full frame.

| Screen | C#, CPU | C#, GPU | Rust, before | Rust, after | Rust (after) vs C# GPU |
|---|---:|---:|---:|---:|---:|
| Custom (showcase.json) | 18.9 | 16.9 | 6.5 | 4.5 | 3.8× |
| Circular | 21.8 | 28.8 | 16.1 | 7.6 | 3.8× |
| Needle | 13.6 | 13.8 | 5.1 | 2.6 | 5.3× |
| Sweep | 22.9 | 20.2 | 11.7 | 5.3 | 3.8× |
| Bar | 21.7 | 22.6 | 4.8 | 2.9 | 7.8× |
| Digital | 12.2 | 11.2 | 5.5 | 3.2 | 3.5× |
| MinMax | 19.4 | 20.7 | 8.7 | 4.5 | 4.6× |
| Histogram | 25.9 | 22.0 | 37.0 | **9.4** | 2.3× |
| Grid | 6.3 | 7.2 | 3.0 | 3.0 | 2.4× |
| TurboXs | 6.7 | 8.9 | 4.4 | 4.4 | 2.0× |
| TurboXs, `"Lcd": true` | 18.2 | 5.2 | 4.7 | 4.6 | 1.1× |

"Before" is the renderer as first measured, "after" is with the aarch64 fixes described below. Both are averages of two runs on the same day, and both produce byte-identical frames.

ms/frame. A run before the GPU mode existed gave the same picture for the old renderer (Rust Circular 22.5, Histogram 37.1).

- **The GPU path is real.** With `--gpu`, every gauge draws on a GPU canvas. The Mali raises about 6,600 interrupts over a TurboXs run, against 27 for the CPU run. Frames read back from the GPU (`--png`) match the CPU frames to within 0.2 % of pixels.
- **The GPU barely helps C#.** Only the LCD look gains much, because it is one shader draw on the GPU. Everywhere else, recording the draw on the CPU and the Mali-G31's own work cost about as much as rasterising on the CPU. During a GPU run the bench uses about one core, and the Mali stays at its lowest clock (420 of 648 MHz), so the time goes to the CPU side (Skia's GL backend and the driver), not to the GPU. In the app, the CPU and GPU overlap across frames, so its frame time is closer to the larger of the two than to their sum, which this per-frame wait measures.
- **Pinning the GPU clock helps the GPU path.** Rerun with the Mali's `min_freq` at 648 MHz and the CPU governor at `performance` (1512 MHz), keeping muOS from idling during the run:

  | Screen | C# GPU, default clocks | C# GPU, max clocks |
  |---|---:|---:|
  | Custom | 16.9 | 12.4 |
  | Circular | 28.8 | 22.1 |
  | Needle | 13.8 | 15.6 |
  | Sweep | 20.2 | 13.2 |
  | Bar | 22.6 | 13.5 |
  | Digital | 11.2 | 9.2 |
  | MinMax | 20.7 | 16.5 |
  | Histogram | 22.0 | 18.6 |
  | Grid | 7.2 | 7.6 |
  | TurboXs | 8.9 | 6.5 |
  | TurboXs, `"Lcd": true` | 5.2 | 3.7 |

  C# on the CPU and Rust didn't change, so the CPU governor makes no difference: `ondemand` already clocks up under load. The gain comes from the GPU clock, which `simple_ondemand` leaves at its lowest step. Even at full clock, the current Rust renderer is faster everywhere except the LCD look (4.6 against 3.7 ms). Before the aarch64 fixes, Histogram was slower too.
- **Rust used to lose most of its lead on the A53.** From a desktop x86 (AMD Ryzen 7 3800X 8-Core Processor, same day) to the device, every C# screen gets 3.5–4.5× slower, but every Rust screen got 8.4–11× slower. Histogram (4.0 → 37.0 ms) was the heaviest screen, so it fell behind C# (27 fps). Profiling on the device (a sampling profiler built into a test binary, then stage timers) found the cause:
  - **Not tiny-skia, and not missing NEON there.** tiny-skia 0.11 enables its NEON paths on aarch64 by default. `-C target-cpu=cortex-a53` changed nothing (within 1 %), and the release profile already has `opt-level = 3`, thin LTO and one codegen unit.
  - **Our own per-pixel loops.** 60–75 % of Histogram, Circular and Sweep went to the glow pipeline in `composite.rs`: the colour blend, the bilinear upscale and the sprite blend. They were written one pixel at a time, with branches and 32-bit math. On aarch64 that compiled to 4-lane code inside a single pixel, with loop state spilled to the stack and a vector-to-register move on the critical path. The upscale had no vector code at all (four gathers per pixel). A wide out-of-order x86 core hides that latency; the in-order A53 can't.
  - **tiny-skia's anti-aliasing cost per row.** It supersamples every pixel row of a path, even a plain rectangle, which costs about 1.4 µs per row on the A53. The 50 crisp Histogram bars took 4.7 ms per frame.
- **What changed** (all exact, checked with randomised tests against the old code and tiny-skia, PNG comparisons on both architectures, and the unit tests run on the device):
  1. The blend loops work on 16-bit lanes without branches, so they vectorise across pixels (`ld4`/`st4` on NEON). Fully transparent or opaque blocks of 16 pixels are skipped or copied.
  2. The upscale is separable (blend the two source rows, then the two columns, which is the same integer sum) and polyphase for the 2× and 4× scales, and it streams each row straight into the blend instead of filling a buffer first.
  3. Axis-aligned rectangle fills let tiny-skia rasterise only their fractional edge strips. The whole-pixel interior is one aligned fill, which tiny-skia blits exactly as it would blit the fully covered runs.
  4. Each glow's blurred low-res coverage is memoised, keyed on the exact low-res path, size and blur (up to 4 MB per rendering thread). Scrolling histogram bars, ring segments and digits recur exactly.
  5. Each glow is only upscaled and blended where its coverage is non-zero. The box blur has compact support, and the glow padding leaves a band of exact zeros around each glow.

  Histogram went from 37.0 to 9.4 ms (4×), Circular from 16.1 to 7.6 ms and Sweep from 11.7 to 5.3 ms. The desktop also got faster, but by less (Histogram 4.4 → 1.4–1.7 ms), so the x86-to-A53 gap narrowed to 6–10×.
- **What didn't help:**
  - `target-cpu=cortex-a53`.
  - Hand-written NEON intrinsics for the colour blend (`umull` + `raddhn` for the exact divide by 255): 8.9 ns/pixel, the same as the compiler's version. A packed-RGBA variant that avoids `ld4`/`st4` was slower (9.7 ns).
- **What's left:**
  - Every built-in screen copies its full-frame static background each frame, about 1.1 ms (1.2 MB at roughly DRAM speed). That's most of Grid's 3 ms. Avoiding it needs damage tracking for the built-in gauges.
  - Histogram's remaining cost is blending 50 overlapping glows, which can't be merged without changing the rounding. The bench uses `"IntervalMs": 0`, so the bars are rebuilt every frame. With the default of 1000 ms (the editor allows 100 ms at least), they're rebuilt at most a few times per second.
  - The custom renderer spawns its scoped threads every frame, which costs about 3 % on the device.

## Caveats

- **CPU only, except section 5.** On the device, the C# app renders through Skia's GPU backend (GLES2), which the VMs can't run. Section 5 measures that with `GaugeBench --gpu` on the device.
- **Noise:** about ±10 % between runs; min–max is shown in section 1.
