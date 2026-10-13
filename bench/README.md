# C# vs Rust renderer benchmark

Complete, current numbers are in [RESULTS.md](RESULTS.md).

`./run.sh` renders every screen in [`rust/examples/gauges.json`](../rust/examples/gauges.json) with both renderers:
- **C#:** the app's real, unmodified renderer sources (linked into [`csharp/`](csharp)) drawing through SkiaSharp 3.119.
- **Rust:** `gauge bench` from [`rust/`](../rust) drawing through tiny-skia.

Both use a 640×480 CPU raster surface by default and the same simulated ECU data stepped at 60 Hz. Each screen gets 300 warm-up frames, then 600 frames where **only the draw call** is timed.

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

"First port" is the initial Rust version; "optimised" adds the changes listed under "How to read this" below. All rows are full repaints every frame; the Custom row was measured before incremental redraw was added, which is covered in its own section below. The C# column is from the same session as the optimised run.

## How to read this

- **The gap is mostly rendering strategy, not language.** An earlier ablation (not committed) forced the first Rust port's glows to full resolution, the way Skia's CPU blur works. That made Rust roughly even with SkiaSharp: faster on light screens, slower on Histogram (16.9 ms). The optimised Rust numbers come from doing less work per frame:
  - **Downscaled blurs:** large glows are blurred at 1/2 or 1/4 resolution, as Skia's GPU backend does.
  - **Alpha-only glows:** a glow is one colour, so only its coverage (A8) is blurred and upscaled. That is ¼ of the RGBA work, and the vertical pass walks rows for cache locality.
  - **Trimmed sprites:** cached layers are cropped to their content, and opaque ones are copied row by row. Previously a full 640×480 mostly-transparent layer was blended per cache per frame.
  - **More caching:** lit segments (Bar, MinMax, Circular), segment-display digits (until the text changes), Histogram bars (until a sample arrives), and runs of static custom elements together with the background.
- **Some of these caches are worst-cased by the benchmark.** It changes every value on every frame. With real ECU data (about 20 Hz against 60 fps) and the default 1 s Histogram interval, most frames hit the caches, so the device will do better than these numbers suggest.
- **On the device, the C# app renders through Skia's GPU backend (GLES2).** `GaugeBench <config> [frames] [warmup] --gpu` measures that path. It opens an SDL window with a GLES2 context, as the app's `GaugeSDL` does, and flushes and waits for the GPU after every timed frame. `--png <dir>` saves each screen's last frame, read back from the surface, to check the GPU output against a CPU run. Device results are in [RESULTS.md](RESULTS.md#5-on-the-device-rg35xx). To run it there, publish with `dotnet publish bench/csharp/GaugeBench.csproj -c Release -r linux-arm64 --self-contained true` and copy it over along with an aarch64 `gauge` build.
- Same machine, alternating runs; expect ±10 % noise between runs.

## Partial redraw and threads (custom gauges)

The table above times full repaints, as the C# app does every frame. The Rust custom-gauge renderer also tracks damage and spreads work over cores; see [rust/README.md](../rust/README.md#performance). Showcase, desktop x86:

| Situation | 1 thread | 4 threads | Repainted |
|---|---:|---:|---:|
| Full repaint (`--full`) | 2.24 ms | 1.46 ms | 100 % |
| Values gliding, new data every frame | 1.36 ms | 0.96 ms | 26 % |
| Values gliding, 20 Hz data (`--data-hz 20`) | 1.36 ms | 1.03 ms | 25 % |
| Values settled (only the graph scrolls) | 0.09 ms | 0.08 ms | 3 % |

The full-repaint figure is above the 1.86 ms in the main table. Making tiled and partial frames match a full render exactly costs extra work: curves are flattened up front, every changed element gets a bounds pass, and glow padding is wider. The partial-redraw and multicore paths more than repay that.

## Where the Rust time goes now

Profiling the custom showcase shows most of the remaining time in tiny-skia's anti-aliased path filling (arc strokes, glyph outlines). That is the actual geometry work. Glow upscale and composite come next. Both of the next steps suggested here, partial redraw and multithreading, have since been done for custom gauges; see the section above.
