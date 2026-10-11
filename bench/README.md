# C# vs Rust renderer benchmark

`./run.sh` renders every screen in [`rust/examples/gauges.json`](../rust/examples/gauges.json) with both renderers:
- **C#:** the app's real, unmodified renderer sources (linked into [`csharp/`](csharp)) drawing through SkiaSharp 3.119.
- **Rust:** `gauge bench` from [`rust/`](../rust) drawing through tiny-skia.

Both use a 640×480 CPU raster surface and the same simulated ECU data stepped at 60 Hz. Each screen gets 300 warm-up frames, then 600 frames where **only the draw call** is timed.

## Results

Intel Xeon @ 2.10 GHz (cloud VM), Linux x64, .NET 10.0.12, Rust 1.97. Each figure is the median of 3 runs.

| Screen | C# ms/frame | Rust, first port | Rust, optimised | Speedup vs C# |
|---|---:|---:|---:|---:|
| Custom (showcase.json, 19 elements) | 8.65 | 3.15 | 1.86 | 4.7× |
| Circular | 10.34 | 4.26 | 1.90 | 5.5× |
| Needle | 6.24 | 1.76 | 0.59 | 10.6× |
| Sweep | 8.65 | 3.46 | 1.43 | 6.0× |
| Bar | 8.92 | 2.20 | 0.56 | 15.9× |
| Digital | 6.16 | 1.70 | 0.73 | 8.5× |
| MinMax | 8.82 | 2.83 | 0.98 | 9.0× |
| Histogram | 10.58 | 7.57 | 3.90 | 2.7× |
| Grid | 2.92 | 0.80 | 0.38 | 7.6× |

| | C# | Rust |
|---|---:|---:|
| Peak RSS (whole benchmark) | 81 MB | 22 MB |
| Deployable size | .NET runtime + app + 11 MB `libSkiaSharp.so` | 2.5 MB static binary (fonts embedded) |

"First port" is the initial Rust version; "optimised" adds the changes listed under "Where the Rust time goes" below. The C# column is from the same session as the optimised run.

## How to read this

- **The gap is mostly rendering strategy, not language.** An earlier ablation (not committed) forced the first Rust port's glows to full resolution, the way Skia's CPU blur works. That made Rust roughly even with SkiaSharp: faster on light screens, slower on Histogram (16.9 ms). The optimised Rust numbers come from doing less work per frame:
  - **Downscaled blurs:** large glows are blurred at 1/2 or 1/4 resolution, as Skia's GPU backend does.
  - **Alpha-only glows:** a glow is one colour, so only its coverage (A8) is blurred and upscaled. That is ¼ of the RGBA work, and the vertical pass walks rows for cache locality.
  - **Trimmed sprites:** cached layers are cropped to their content, and opaque ones are copied row by row. Previously a full 640×480 mostly-transparent layer was blended per cache per frame.
  - **More caching:** lit segments (Bar, MinMax, Circular), segment-display digits (until the text changes), Histogram bars (until a sample arrives), and runs of static custom elements together with the background.
- **Some of these caches are worst-cased by the benchmark.** It changes every value on every frame. With real ECU data (about 20 Hz against 60 fps) and the default 1 s Histogram interval, most frames hit the caches, so the device will do better than these numbers suggest.
- **On the device, the C# app renders through Skia's GPU backend (GLES2), which this benchmark does not cover.** The cloud VM has no GPU. What matters there is how the RG35XX's GPU compares with the Rust renderer on its Cortex-A53 cores. Run `gauge bench` on the device for that.
- Same machine, alternating runs; expect ±10 % noise between runs.

## Where the Rust time goes now

Profiling the custom showcase shows most of the remaining time in tiny-skia's anti-aliased path filling (arc strokes, glyph outlines). That is the actual geometry work. Glow upscale and composite come next. Further gains would need structural changes:
- dirty-region redraw, which only repaints the bounds of elements whose value changed;
- multithreading independent elements.
