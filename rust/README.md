# GaugeDotnet — Rust renderer

A Rust port of GaugeDotnet's rendering stack. It loads the same `custom-gauge.json` files the Avalonia designer saves, and the same `gauges.json` screen list. It draws them on the CPU with [tiny-skia](https://github.com/RazrFalcon/tiny-skia), and the frames can be shown in an SDL2 window or saved as PNG.

![showcase](examples/showcase.png)

## Layout

| Crate | Purpose |
|-------|---------|
| `gauge-render` | Library: canvas, fonts, the 17 custom element renderers, calculated channels, the 8 built-in gauges and the `gauges.json` factory |
| `gauge-app` | `gauge` binary: PNG snapshots, a benchmark and an optional SDL2 window, all driven by a port of `SimulatedMeDevice` |

### C# → Rust map

| C# | Rust |
|----|------|
| `Gauges/Custom/GaugeElement.cs` | `model.rs` (serde, the same `$type` names and PascalCase properties) |
| `Gauges/Custom/ElementRenderer.cs` + `Renderers/*` | `custom/renderer.rs` (`ElementRenderer`) |
| `Gauges/Custom/CustomGauge.cs` | `custom/gauge.rs` (frame-rate independent smoothing) |
| `Gauges/Custom/ExpressionEvaluator.cs` | `expression.rs` |
| `ImageCache` / `GifCache` | `images.rs` (PNG, JPEG, WebP, plus GIF/APNG/WebP animation frames) |
| `Rendering/FontHelper.cs` | `text.rs` (the four bundled fonts are embedded in the binary) |
| `BaseGauge`, `BarGauge`, `CircularGauge`, `NeedleGauge`, `SweepGauge`, `DigitalGauge`, `MinMaxGauge`, `HistogramGauge`, `GridGauge`, `SegmentDisplay` | `gauges/*.rs` |
| `GaugeFactory`, `AppConfig`, `GaugeConfig`, `TolerantGaugeTypeConverter` | `gauges/mod.rs`, `gauges/config.rs` |
| `SKCanvas` / `SKPaint` / `SKMaskFilter` | `canvas.rs` (save/restore, transforms, clips, `SaveLayer` opacity, arcs, round rects, text, normal and outer blur) |
| `value.ToString("F1")`, `DateTime.ToString("HH:mm")` | `format.rs` (standard and custom .NET numeric and date format strings, invariant culture) |
| `SimulatedMeDevice` | `gauge-app/src/sim.rs` |
| `GaugeSDL` / `GameLoop` | `gauge-app/src/window.rs` |

## Build and run

```shell
cd rust
cargo test                                    # unit + render tests
cargo run --release -- render examples/showcase.json -o out.png
cargo run --release -- render examples/gauges.json -o screen.png     # one PNG per screen
cargo run --release -- render my-gauge.json --set Rpm=6500 --set OilTemp=104
cargo run --release -- builtin needle --value 72 --title SPEED --unit KMH --color '#FF8800'
cargo run --release -- bench examples/showcase.json

# Live window (needs libsdl2-dev). Left/Right or the D-pad switch screens; Esc quits.
cargo run --release --features sdl -- run examples/gauges.json
```

Relative paths inside a definition (images, needle images, GIFs) resolve against the JSON file's folder, as in the C# app.

### Using the library

```rust
use gauge_render::{Canvas, CustomGauge};

let mut gauge = CustomGauge::load("custom-gauge.json")?;
let mut canvas = Canvas::new(640, 480);
loop {
    gauge.update_from(|source| ecu.read(source)); // e.g. "Rpm", "OilTemp"
    gauge.draw(&mut canvas);                      // smooths toward the targets, then renders
    present(canvas.pixmap().data());              // RGBA8, premultiplied (opaque frames)
}
```

### Cross-compiling for the RG35XX (aarch64)

```shell
rustup target add aarch64-unknown-linux-gnu
sudo apt install gcc-aarch64-linux-gnu
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
  cargo build --release --target aarch64-unknown-linux-gnu -p gauge-app
```

That produces a headless build: `render` and `bench` work, but there is no window. The SDL window (`--features sdl`) also needs the device's `libSDL2` to link against, from a sysroot or by building on the device. The headless aarch64 binary has been checked under `qemu-aarch64`, and its output matches the x86 build pixel for pixel.

## Performance

Everything is rasterised on the CPU. Glows are the expensive part, so:
- **Large blurs run downscaled:** sigmas of 3 or more are blurred on a 2× or 4× smaller buffer and upscaled bilinearly, the way Skia's GPU backend does.
- **Glows are alpha-only:** each glow is one colour, so only its coverage (A8) is rasterised and blurred, then composited with that colour.
- **Static content is cached as trimmed `Sprite`s.** The built-in gauges cache their backgrounds, lit segments (keyed by count and colour), segment-display digits (keyed by text and colour) and Histogram bars (keyed by sample version). `CustomGauge` pre-renders each run of static elements, together with the background (`ElementRenderer::render_cached`).
- **Opacity layers composite only the area that was drawn into.**

`bench examples/showcase.json` (19 elements covering 15 of the 17 element types, with glows) takes about 1.9 ms per frame on a desktop x86 core. The C# renderer takes 8.7 ms on the same CPU; see [`bench/`](../bench/README.md). That leaves plenty of room for 60 fps on the RG35XX's Cortex-A53, but measure on the device with `gauge bench`.

## Differences from the C# renderer

- **CPU rendering** instead of Skia on GLES2. Antialiasing and blur falloff differ slightly: the gaussian is approximated with three box blurs.
- **Calculated-channel inputs are tracked.** The C# `CustomGauge` only polls sources bound directly to an element, so `Rpm / 1000` read 0 unless some element also used `Rpm`. Visibility sources are tracked too.
- **Invalid colours render black** instead of throwing from `SKColor.Parse`. Likewise, the `D`/`X` number formats render a rounded integer, where .NET would throw for a float.
- **No global state.** Graph history, peak-hold values and image caches live in each `ElementRenderer`, not in static dictionaries.
- `GraphElement.HistoryDepth` below 2 is treated as 2. In C# it divides by zero.

Not ported, because they are outside the renderer: BLE and CAN decoding (`ME1_4NET`), the Avalonia designer, the on-device config editor, splash, battery and error overlays, and gamepad input.
