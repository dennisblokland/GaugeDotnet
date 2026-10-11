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

## 3. Resources

| | C# | Rust |
|---|---:|---:|
| Peak RSS, whole benchmark | 81 MB | 24 MB |
| Deployable size | .NET runtime + app + 11 MB `libSkiaSharp.so` | 2.6 MB stripped binary (fonts embedded) |

## Caveats

- **CPU only.** On the device, the C# app renders through Skia's GPU backend (GLES2), which this VM can't run. The relevant device comparison is RG35XX GPU vs Rust on its Cortex-A53 cores; use `gauge bench` there.
- **Noise:** about ±10 % between runs; min–max is shown in section 1.
