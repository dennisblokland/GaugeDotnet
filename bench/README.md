# C# vs Rust renderer benchmark

`./run.sh` renders every screen in [`rust/examples/gauges.json`](../rust/examples/gauges.json) with both renderers:
- **C#:** the app's real, unmodified renderer sources (linked into [`csharp/`](csharp)) drawing through SkiaSharp 3.119.
- **Rust:** `gauge bench` from [`rust/`](../rust) drawing through tiny-skia.

Both use a 640×480 CPU raster surface and the same simulated ECU data stepped at 60 Hz. Each screen gets 300 warm-up frames, then 600 frames where **only the draw call** is timed.

## Results

Intel Xeon @ 2.10 GHz (cloud VM), Linux x64, .NET 10.0.12, Rust 1.97. Each figure is the median of 3 runs.

| Screen | C# ms/frame | Rust ms/frame | Rust speedup | Rust with full-res blur¹ |
|---|---:|---:|---:|---:|
| Custom (showcase.json, 19 elements) | 8.30 | 3.15 | 2.6× | 6.2 |
| Circular | 10.30 | 4.26 | 2.4× | 7.7 |
| Needle | 6.12 | 1.76 | 3.5× | 3.0 |
| Sweep | 7.97 | 3.46 | 2.3× | 8.5 |
| Bar | 8.77 | 2.20 | 4.0×² | 3.2 |
| Digital | 6.12 | 1.70 | 3.6× | 2.5 |
| MinMax | 8.47 | 2.83 | 3.0×² | 5.1 |
| Histogram | 9.77 | 7.57 | 1.3× | 16.9 |
| Grid | 2.83 | 0.80 | 3.5× | 0.7 |

| | C# | Rust |
|---|---:|---:|
| Peak RSS (whole benchmark) | 81 MB | 22 MB |
| Deployable size | .NET runtime + app + 11 MB `libSkiaSharp.so` | 2.5 MB static binary (fonts embedded) |

¹ An ablation run (not committed) that forced the Rust blur to full resolution, the way Skia's CPU blur works.
² Partly algorithmic: the Rust Bar and MinMax gauges cache their lit segments (as C#'s `CircularGauge` already does). The C# versions re-blur every lit segment on every frame.

## How to read this

- **Most of the Rust lead comes from the blur strategy, not the language.** Glows dominate the frame cost. The Rust canvas blurs large sigmas on a 2×/4× downscaled buffer, as Skia's GPU backend does. Forced to full resolution, Rust is roughly on par with Skia's SIMD-optimised CPU blur:
  - faster on Needle, Digital, Bar and Grid;
  - about the same on Custom, Circular and Sweep;
  - slower on Histogram, which draws 50 separately blurred bars.
- **Without glows, Rust is faster** (Grid, which has no blur: 0.8 vs 2.8 ms). That comes from lower per-call overhead: no P/Invoke boundary, plus integer compositing.
- **On the device, the C# app renders through Skia's GPU backend (GLES2), which this benchmark does not cover.** The cloud VM has no GPU. What matters there is how the RG35XX's GPU compares with the Rust renderer on its Cortex-A53 cores. Run `gauge bench` on the device for that.
- Same machine, alternating runs; expect ±10 % noise between runs.
