# GaugeDotnet

Real-time automotive gauge display for the [RG35XX](https://anbernic.com) handheld (ARM64 Linux). Connects via BLE to a Motorsports Electronics ECU, decodes CAN bus frames, and renders live gauges with SkiaSharp on SDL2 + OpenGL ES 2.0.

![.NET 10](https://img.shields.io/badge/.NET-10-512BD4) ![Platform](https://img.shields.io/badge/platform-linux--arm64-green) ![License](https://img.shields.io/badge/license-MIT-blue)

## Features

- Live gauge rendering at 60 fps via SDL2 + OpenGL ES 2.0
- BLE connection to Motorsports Electronics ECU (CAN bus frames ME1_1–ME1_8)
- Multiple built-in gauge styles: circular, sweep, needle, bar, digital, histogram, min/max, grid
- **Custom gauge designer** — drag-and-drop Avalonia desktop app that exports JSON definitions for the device
- 11 custom element types: arcs, needles (with image support), text, value displays, tick rings, bars, warning indicators, images, and more
- Smooth value interpolation (lerp) for fluid needle/arc/bar animation
- Simulated device mode for desktop development (`SimulatedMeDevice` in DEBUG builds)

## Projects

| Project | Purpose |
|---------|---------|
| `GaugeDotnet` | Main app — SDL2 window, gauge rendering, BLE connection |
| `GaugeDotnet.Designer` | Avalonia 12 drag-and-drop gauge designer (desktop) |
| `ME1_4NET` | CAN bus decoder library (Motorsports Electronics frames ME1_1–ME1_8) |
| `ME1_4NET.Tests` | xUnit tests for CAN frame decoding |
| `RG35XX.Core` | Platform abstractions (gamepad, storage interfaces) |
| `RG35XX.Libraries` | Linux implementations (joystick, keyboard, storage) |

## Prerequisites

- [.NET 10 SDK](https://dotnet.microsoft.com/download)
- SDL2 development libraries (`libsdl2-dev` on Debian/Ubuntu)
- A GitHub PAT with `read:packages` scope (for the `VaettirNet.Btleplug` NuGet package)

## Setup

The `VaettirNet.Btleplug` BLE library is sourced from GitHub Packages. Set the PAT before restoring:

```shell
export GITHUB_PACKAGES_PAT=ghp_your_token_here
dotnet restore
```

See [nuget.config](nuget.config) for the package source configuration.

## Build & Run

```shell
dotnet build                                    # Build all projects
dotnet test                                     # Run xUnit tests
dotnet run --project src/GaugeDotnet            # Run app (SimulatedMeDevice in DEBUG)
dotnet run --project src/GaugeDotnet.Designer   # Open the gauge designer
```

### Deploy to Device

```shell
./deploy-arm64.sh
```

Publishes a self-contained linux-arm64 binary and SCPs it to the RG35XX.

## Architecture

```
┌─────────────────────────────────────────────┐
│                  GameLoop                    │
│  ┌──────────┐  ┌──────────┐  ┌───────────┐ │
│  │ GaugeSDL │  │InputHandler│ │ FpsCounter│ │
│  │ SDL2+GL  │  │ Gamepad+KB │ │           │ │
│  └────┬─────┘  └───────────┘  └───────────┘ │
│       │                                      │
│  ┌────▼──────────────────────────────────┐   │
│  │           BaseGauge / CustomGauge     │   │
│  │  ┌─────────────────────────────────┐  │   │
│  │  │      ElementRenderer (Skia)     │  │   │
│  │  └─────────────────────────────────┘  │   │
│  └───────────────────────────────────────┘   │
│       │                                      │
│  ┌────▼─────┐  ┌──────────────────────────┐ │
│  │ IMeDevice│  │      CanDecoder          │ │
│  │ BLE/Sim  │  │  ME1_1..ME1_8 → MEData  │ │
│  └──────────┘  └──────────────────────────┘ │
└─────────────────────────────────────────────┘
```

- **GaugeSDL** creates the SDL2 window with OpenGL ES 2.0 and wraps it with a SkiaSharp `GRContext`/`SKSurface`
- **BaseGauge** subclasses use a static cache pattern: inactive segments drawn once to bitmap, active segments redrawn per-frame
- **MeDevice** / **SimulatedMeDevice** implement `IMeDevice` — `#if DEBUG` uses the simulated device to bypass BLE
- **CanDecoder** dispatches CAN frames by PID to typed structs; `MEData` aggregates all decoded values

## Custom Gauge System

Design gauges on your desktop, deploy JSON to the device.

### Workflow

1. Run the designer: `dotnet run --project src/GaugeDotnet.Designer`
2. Add and arrange elements, bind data sources, tweak properties
3. Save → `custom-gauge.json`
4. Reference in your `gauges.json`:
   ```json
   { "Gauge": { "Type": "Custom" }, "CustomDefinitionFile": "custom-gauge.json" }
   ```

### Element Types

| Type | JSON `$type` | Data-Driven | Description |
|------|-------------|-------------|-------------|
| Arc | `arc` | Yes | Sweep arc with track, glow, dynamic fill |
| Needle | `needle` | Yes | Rotating needle with tail + hub (optional image) |
| Text | `text` | No | Static label with custom font |
| Value Display | `value` | Yes | Formatted numeric display |
| Tick Ring | `ticks` | No | Major/minor tick marks with labels |
| Circle | `circle` | No | Filled/stroked circle |
| Rectangle | `rectangle` | No | Rectangle with optional corner radius |
| Line | `line` | No | Straight line between two points |
| Linear Bar | `linearbar` | Yes | Horizontal/vertical fill bar |
| Warning Indicator | `warning` | Yes | Threshold-triggered color change with label |
| Image | `image` | No | Background, logo, or overlay with opacity + rotation |

### Needle Image Support

Needle elements can use a custom image instead of a drawn line. The image should be a vertical strip pointing **up** (12 o'clock) with the pivot at the bottom edge. The renderer rotates it around the element's (X, Y) point.

For a 640×480 canvas, a needle image of roughly 20×180 px works well. Use a transparent PNG for best results.

### Value Smoothing

Display values are interpolated toward the target at 10% per frame for fluid animation. Visual elements (arc, needle, bar) use the smoothed value; text/value/warning elements snap to the actual value.

## Tests

```shell
dotnet test
```

xUnit tests verify CAN frame decoding against raw byte array payloads using the Arrange/Act/Assert pattern.

## License

See [LICENSE](LICENSE) for details.
