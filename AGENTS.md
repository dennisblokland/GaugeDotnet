# GaugeDotnet — Agent Instructions

Real-time automotive gauge display for the RG35XX handheld (ARM64 Linux). Connects via BLE to a MegaSquirt ECU, decodes CAN bus frames, and renders live gauges with SkiaSharp on SDL2 + OpenGL ES 2.0.

## Projects

| Project | Purpose |
|---------|---------|
| `GaugeDotnet` | Main app — SDL2 window, gauge rendering, BLE connection |
| `GaugeDotnet.Designer` | Avalonia 12 drag-and-drop gauge designer (desktop) |
| `ME1_4NET` | CAN bus decoder library (MegaSquirt frames `ME1_1`–`ME1_8`) |
| `ME1_4NET.Tests` | xUnit tests for CAN frame decoding |
| `RG35XX.Core` | Platform abstractions (gamepad, storage interfaces) |
| `RG35XX.Libraries` | Linux implementations (joystick, keyboard, storage) |

## Build & Test

```shell
dotnet build              # Build all projects
dotnet test               # Run xUnit tests
dotnet run --project src/GaugeDotnet  # Run app (uses SimulatedMeDevice in DEBUG)
```

Deploy to device: `./deploy-arm64.sh` (publishes linux-arm64, SCPs to device)

## Setup

`VaettirNet.Btleplug` is sourced from GitHub Packages. Set the `GITHUB_PACKAGES_PAT` environment variable to a GitHub PAT with `read:packages` scope before restoring. See [nuget.config](nuget.config).

## Conventions

- **Framework:** .NET 10, central package management via [Directory.Packages.props](Directory.Packages.props)
- **Nullable:** enabled in all projects
- **Indentation:** tabs, size 4 (see [editorconfig](editorconfig))
- **Private fields:** `_camelCase` prefix — enforced by editorconfig
- **`var` usage:** prefer explicit types (`var` discouraged)
- **Braces:** always use braces (suggestion level)
- **`AllowUnsafeBlocks`** is enabled in GaugeDotnet (SDL2/OpenGL interop)

## Architecture Notes

- `GaugeSDL` — Creates SDL2 window with OpenGL ES 2.0, wraps with SkiaSharp `GRContext`/`SKSurface`
- `BaseGauge`/`BarGauge` — Static cache pattern: inactive segments drawn once to bitmap, active segments redrawn per-frame
- `MeDevice`/`SimulatedMeDevice` implement `IMeDevice` — `#if DEBUG` uses `SimulatedMeDevice` to bypass BLE
- `CanDecoder` dispatches CAN frames by `Pid` to typed structs; `MEData` aggregates decoded values
- Font files in `src/GaugeDotnet/fonts/` are copied to output at build time

## Custom Gauge System

Drag-and-drop gauge designer → JSON definition → runtime rendering on device.

### Workflow

1. `dotnet run --project src/GaugeDotnet.Designer` — opens Avalonia designer
2. Add/arrange elements, bind data sources, tweak properties
3. Save → `custom-gauge.json`
4. Reference in `gauges.json`: `{ "Gauge": { "Type": "Custom" }, "CustomDefinitionFile": "custom-gauge.json" }`

### Element Types

| Type | `$type` | Data-Driven | Description |
|------|---------|-------------|-------------|
| `ArcElement` | `arc` | Yes | Sweep arc with track, glow, dynamic fill |
| `NeedleElement` | `needle` | Yes | Rotating needle with tail + hub (optional image needle) |
| `TextElement` | `text` | No | Static label (custom font) |
| `ValueDisplayElement` | `value` | Yes | Formatted numeric display |
| `TickRingElement` | `ticks` | No | Major/minor tick marks with labels |
| `CircleElement` | `circle` | No | Filled/stroked circle |
| `RectangleElement` | `rectangle` | No | Rectangle with optional corner radius |
| `LineElement` | `line` | No | Straight line between two points |
| `LinearBarElement` | `linearbar` | Yes | Horizontal/vertical fill bar |
| `WarningIndicatorElement` | `warning` | Yes | Threshold-triggered color change with label |
| `ImageElement` | `image` | No | Custom image (backgrounds, logos, overlays) with opacity + rotation |

`CustomGaugeDefinition` also supports `BackgroundImage` (path) drawn behind all elements.

### Key Files

- `Gauges/Custom/GaugeElement.cs` — Polymorphic element models (`[JsonDerivedType]`)
- `Gauges/Custom/ElementRenderer.cs` — Core SkiaSharp draw methods (shared by app + designer)
- `Gauges/Custom/CustomGauge.cs` — `BaseGauge` subclass, smoothing (lerp), source-gen JSON context
- `Configuration/DataSourceMapper.cs` — Maps string names → `MEData` fields
- `GaugeDotnet.Designer/Rendering/ElementRenderer.cs` — Designer wrapper (hit-test, selection highlights)
- `GaugeDotnet.Designer/MainWindow.axaml(.cs)` — Full designer UI

### Smoothing

`CustomGauge` lerps display values toward target at 10%/frame. Visual elements (arc, needle, bar) use smoothed values. Text/value/warning elements snap to target values.

### Needle Image Guide

When using `ImagePath` on a `NeedleElement`, the image is rotated around the element's (X, Y) pivot point. The image should be a vertical needle pointing **up** (12 o'clock), as the renderer applies rotation from that orientation.

| Property | Description | Recommendation |
|----------|-------------|----------------|
| `ImagePath` | Path to PNG/JPG/WebP (relative to JSON or absolute) | Use transparent PNG for best results |
| `ImageWidth` | Rendered width of the needle image (px) | Match the visual width of the needle artwork |
| `ImageLength` | Rendered length from pivot to tip (px) | Should match `Length` or the visible needle portion |
| `Length` | Still used for hit-test bounds when image is set | Keep equal to `ImageLength` |

**Image orientation:** The source image should be a vertical strip with the **tip at the top** and the **pivot point at the bottom edge**. The renderer draws it into a rect from `(-ImageWidth/2, -ImageLength)` to `(ImageWidth/2, 0)` relative to the pivot, then rotates.

**Recommended dimensions:** For a 640×480 gauge canvas, a needle image of roughly 20×180 px works well. Use a tall narrow PNG (e.g., 40×360 source scaled down to `ImageWidth: 20, ImageLength: 180`).

## Tests

- xUnit with `[Fact]`, Arrange/Act/Assert pattern
- Tests use raw byte array payloads to verify CAN frame decoding
