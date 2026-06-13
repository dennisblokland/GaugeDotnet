# Graph Report - .  (2026-05-22)

## Corpus Check
- Corpus is ~36,278 words - fits in a single context window. You may not need a graph.

## Summary
- 768 nodes · 1045 edges · 97 communities (27 shown, 70 thin omitted)
- Extraction: 98% EXTRACTED · 2% INFERRED · 0% AMBIGUOUS · INFERRED: 16 edges (avg confidence: 0.91)
- Token cost: 3,200 input · 2,800 output

## Community Hubs (Navigation)
- [[_COMMUNITY_Gauge Designer UI|Gauge Designer UI]]
- [[_COMMUNITY_BLE Device Connection|BLE Device Connection]]
- [[_COMMUNITY_Core App & Input|Core App & Input]]
- [[_COMMUNITY_Gauge Architecture|Gauge Architecture]]
- [[_COMMUNITY_Configuration & JSON|Configuration & JSON]]
- [[_COMMUNITY_Gauge Settings Models|Gauge Settings Models]]
- [[_COMMUNITY_Game Loop & Config|Game Loop & Config]]
- [[_COMMUNITY_Config Editor Screen|Config Editor Screen]]
- [[_COMMUNITY_Bluetooth Hardware Init|Bluetooth Hardware Init]]
- [[_COMMUNITY_Custom Gauge Elements|Custom Gauge Elements]]
- [[_COMMUNITY_Built-in Gauges|Built-in Gauges]]
- [[_COMMUNITY_Data Source & Expression|Data Source & Expression]]
- [[_COMMUNITY_Graph & Image Rendering|Graph & Image Rendering]]
- [[_COMMUNITY_Render Context & FPS|Render Context & FPS]]
- [[_COMMUNITY_Custom & Histogram Gauge|Custom & Histogram Gauge]]
- [[_COMMUNITY_CAN Decoder Tests|CAN Decoder Tests]]
- [[_COMMUNITY_Grid Gauge & Config|Grid Gauge & Config]]
- [[_COMMUNITY_MEData Tests|MEData Tests]]
- [[_COMMUNITY_Element Renderer Core|Element Renderer Core]]
- [[_COMMUNITY_BLE Manager|BLE Manager]]
- [[_COMMUNITY_Linux Storage|Linux Storage]]
- [[_COMMUNITY_Segment Display|Segment Display]]
- [[_COMMUNITY_BLE Types & Keep-Alive|BLE Types & Keep-Alive]]
- [[_COMMUNITY_Base Gauge Drawing|Base Gauge Drawing]]
- [[_COMMUNITY_Element Bounds & Selection|Element Bounds & Selection]]
- [[_COMMUNITY_Element Draw Pipeline|Element Draw Pipeline]]
- [[_COMMUNITY_ME1_4 Frame Tests|ME1_4 Frame Tests]]
- [[_COMMUNITY_Bar Gauge|Bar Gauge]]
- [[_COMMUNITY_MinMax Gauge|MinMax Gauge]]
- [[_COMMUNITY_ME1_3 Frame Tests|ME1_3 Frame Tests]]
- [[_COMMUNITY_ME1_5 Frame Tests|ME1_5 Frame Tests]]
- [[_COMMUNITY_ME1_7 Frame Tests|ME1_7 Frame Tests]]
- [[_COMMUNITY_ME1_8 Frame Tests|ME1_8 Frame Tests]]
- [[_COMMUNITY_Gauge Factory|Gauge Factory]]
- [[_COMMUNITY_Circular Gauge|Circular Gauge]]
- [[_COMMUNITY_WSL Launch Config|WSL Launch Config]]
- [[_COMMUNITY_ME1_6 Frame Tests|ME1_6 Frame Tests]]
- [[_COMMUNITY_List Extensions|List Extensions]]
- [[_COMMUNITY_String Extensions|String Extensions]]
- [[_COMMUNITY_ME1_2 Frame Tests|ME1_2 Frame Tests]]
- [[_COMMUNITY_System Env Variables|System Env Variables]]
- [[_COMMUNITY_Font Helper|Font Helper]]
- [[_COMMUNITY_App Launcher|App Launcher]]
- [[_COMMUNITY_ME1_1 Frame Tests|ME1_1 Frame Tests]]
- [[_COMMUNITY_System Utilities|System Utilities]]
- [[_COMMUNITY_Avalonia App Init|Avalonia App Init]]
- [[_COMMUNITY_Gamepad Key Extensions|Gamepad Key Extensions]]
- [[_COMMUNITY_Shape Renderer|Shape Renderer]]
- [[_COMMUNITY_Text Renderer|Text Renderer]]
- [[_COMMUNITY_Error Screen|Error Screen]]
- [[_COMMUNITY_Device Info|Device Info]]
- [[_COMMUNITY_SKCanvas Extensions|SKCanvas Extensions]]
- [[_COMMUNITY_ME1_4NET Examples|ME1_4NET Examples]]
- [[_COMMUNITY_Gamepad Extensions|Gamepad Extensions]]
- [[_COMMUNITY_ReadOnlyList Extensions|ReadOnlyList Extensions]]
- [[_COMMUNITY_Main Program (Avalonia)|Main Program (Avalonia)]]
- [[_COMMUNITY_Main Program (RG35XX)|Main Program (RG35XX)]]
- [[_COMMUNITY_CAN Decoder|CAN Decoder]]
- [[_COMMUNITY_Arc Renderer|Arc Renderer]]
- [[_COMMUNITY_Image Renderer|Image Renderer]]
- [[_COMMUNITY_IMeDevice Interface|IMeDevice Interface]]
- [[_COMMUNITY_MEData Model|MEData Model]]
- [[_COMMUNITY_IStorageProvider Interface|IStorageProvider Interface]]
- [[_COMMUNITY_Local Settings|Local Settings]]
- [[_COMMUNITY_ME1_3 Frame|ME1_3 Frame]]
- [[_COMMUNITY_ME1_5 Frame|ME1_5 Frame]]
- [[_COMMUNITY_ME1_6 Frame|ME1_6 Frame]]
- [[_COMMUNITY_Base Gauge Settings|Base Gauge Settings]]
- [[_COMMUNITY_Indicator Renderer|Indicator Renderer]]
- [[_COMMUNITY_Linear Bar Renderer|Linear Bar Renderer]]
- [[_COMMUNITY_Needle Renderer|Needle Renderer]]
- [[_COMMUNITY_Tick Ring Renderer|Tick Ring Renderer]]
- [[_COMMUNITY_Gauge Config|Gauge Config]]
- [[_COMMUNITY_Grid Cell Config|Grid Cell Config]]
- [[_COMMUNITY_Screen Config|Screen Config]]
- [[_COMMUNITY_ME1_1 Frame|ME1_1 Frame]]
- [[_COMMUNITY_ME1_2 Frame|ME1_2 Frame]]
- [[_COMMUNITY_ME1_4 Frame|ME1_4 Frame]]
- [[_COMMUNITY_ME1_7 Frame|ME1_7 Frame]]
- [[_COMMUNITY_ME1_8 Frame|ME1_8 Frame]]
- [[_COMMUNITY_ICanFrame Interface|ICanFrame Interface]]
- [[_COMMUNITY_Linux Gamepad|Linux Gamepad]]
- [[_COMMUNITY_Launch Config|Launch Config]]
- [[_COMMUNITY_Tasks Config|Tasks Config]]
- [[_COMMUNITY_Connection State|Connection State]]
- [[_COMMUNITY_Designer Window|Designer Window]]
- [[_COMMUNITY_RG35XX Platform|RG35XX Platform]]
- [[_COMMUNITY_CAN PID|CAN PID]]
- [[_COMMUNITY_Gauge Type Enum|Gauge Type Enum]]
- [[_COMMUNITY_Joystick Input|Joystick Input]]
- [[_COMMUNITY_TickRing Element|TickRing Element]]
- [[_COMMUNITY_Circle Element|Circle Element]]
- [[_COMMUNITY_Rectangle Element|Rectangle Element]]
- [[_COMMUNITY_Line Element|Line Element]]
- [[_COMMUNITY_Image Element|Image Element]]

## God Nodes (most connected - your core abstractions)
1. `MainWindow` - 48 edges
2. `ConfigEditor` - 26 edges
3. `GameLoop` - 20 edges
4. `MeDevice` - 19 edges
5. `BluetoothHardwareInit` - 19 edges
6. `MinMaxGauge` - 17 edges
7. `GridGauge` - 16 edges
8. `HistogramGauge` - 16 edges
9. `SegmentDisplay` - 16 edges
10. `GaugeElement` - 16 edges

## Surprising Connections (you probably didn't know these)
- `Dependabot NuGet Weekly Updates` --references--> `GaugeDotnet Main App`  [INFERRED]
  .github/dependabot.yml → AGENTS.md
- `GaugeDotnet Project Overview` --references--> `GaugeDotnet Main App`  [EXTRACTED]
  README.md → AGENTS.md
- `deploy-arm64.sh` --references--> `GaugeDotnet Main App`  [EXTRACTED]
  README.md → AGENTS.md
- `CI Build Job` --references--> `ME1_4NET.Tests`  [INFERRED]
  .github/workflows/CI.yml → AGENTS.md
- `GameLoop` --references--> `GaugeSDL`  [EXTRACTED]
  README.md → AGENTS.md

## Hyperedges (group relationships)
- **Custom Gauge Rendering Pipeline** — gaugedotnet_agents_customgaugedefinition, gaugedotnet_agents_elementrenderer, gaugedotnet_agents_gaugeelement, gaugedotnet_agents_datasourcemapper [INFERRED 0.85]
- **BLE to CAN Data Aggregation Flow** — gaugedotnet_agents_medevice, gaugedotnet_agents_candecoder, gaugedotnet_agents_medata [EXTRACTED 1.00]
- **CI Build, Restore, Test Pipeline** — workflows_ci_yml_ci_workflow, workflows_ci_yml_build_job, workflows_ci_yml_pat_read_packages [EXTRACTED 1.00]

## Communities (97 total, 70 thin omitted)

### Community 0 - "Gauge Designer UI"
Cohesion: 0.16
Nodes (5): GaugeDesignerViewModel, MainWindow, HashSet, Point, Window

### Community 1 - "BLE Device Connection"
Cohesion: 0.06
Nodes (17): BtlePeripheral, CancellationTokenSource, GaugeDotnet.Devices, MeDevice, GaugeDotnet.Devices, SimulatedMeDevice, GRContext, IDisposable (+9 more)

### Community 2 - "Core App & Input"
Cohesion: 0.05
Nodes (15): byte, ConcurrentQueue, FileStream, GaugeDotnet, InputHandler, IGamePadReader, GamePadReader, RG35XX.Libraries (+7 more)

### Community 3 - "Gauge Architecture"
Cohesion: 0.08
Nodes (27): BarGauge, BaseGauge, Calculated Channels (Virtual Data Sources), CanDecoder, CustomGauge, CustomGaugeDefinition, DataSourceMapper, ExpressionEvaluator (+19 more)

### Community 4 - "Configuration & JSON"
Cohesion: 0.09
Nodes (7): AppConfigJsonContext, ConfigService, GaugeDotnet.Configuration, CustomGaugeJsonContext, GaugeDesignerViewModel, JsonSerializerContext, JsonSerializerOptions

### Community 5 - "Gauge Settings Models"
Cohesion: 0.08
Nodes (17): BaseGaugeSettings, BarGaugeSettings, GaugeDotnet.Gauges.Models, CircularGaugeSettings, GaugeDotnet.Gauges.Models, DigitalGaugeSettings, GaugeDotnet.Gauges.Models, GaugeDotnet.Gauges.Models (+9 more)

### Community 6 - "Game Loop & Config"
Cohesion: 0.12
Nodes (10): AppConfig, ConfigEditor, double, FpsCounter, GameLoop, GaugeDotnet, GaugeSDL, InputHandler (+2 more)

### Community 7 - "Config Editor Screen"
Cohesion: 0.19
Nodes (3): EditorScreen, ConfigEditor, GaugeDotnet

### Community 9 - "Custom Gauge Elements"
Cohesion: 0.19
Nodes (18): ArcElement, CalculatedChannel, CircleElement, CustomGaugeDefinition, GaugeElement, GraphElement, ImageElement, LabelValueElement (+10 more)

### Community 10 - "Built-in Gauges"
Cohesion: 0.16
Nodes (9): BaseGauge, DigitalGauge, GaugeDotnet.Gauges, GaugeDotnet.Gauges, NeedleGauge, GaugeDotnet.Gauges, SweepGauge, SegmentDisplay (+1 more)

### Community 11 - "Data Source & Expression"
Cohesion: 0.21
Nodes (5): DataSourceMapper, GaugeDotnet.Configuration, ExpressionEvaluator, ExprParser, Dictionary

### Community 12 - "Graph & Image Rendering"
Cohesion: 0.15
Nodes (4): ConcurrentDictionary, GraphRenderer, ImageCache, PeakMarkerRenderer

### Community 13 - "Render Context & FPS"
Cohesion: 0.15
Nodes (6): RenderContext, FpsCounter, GaugeDotnet.Rendering, SKFont, SKPaint, Stopwatch

### Community 14 - "Custom & Histogram Gauge"
Cohesion: 0.18
Nodes (6): CustomGauge, CustomGaugeDefinition, float, GaugeDotnet.Gauges, HistogramGauge, long

### Community 16 - "Grid Gauge & Config"
Cohesion: 0.18
Nodes (6): AppConfig, GaugeDotnet.Configuration, GaugeDotnet.Gauges, GridGauge, int, List

### Community 18 - "Element Renderer Core"
Cohesion: 0.23
Nodes (12): ArcElement, ElementRenderer (SkiaSharp), GaugeElement (Polymorphic Base), GraphElement, LabelValueElement, LinearBarElement, NeedleElement, PeakMarkerElement (+4 more)

### Community 19 - "BLE Manager"
Cohesion: 0.33
Nodes (3): BtleManager, BleManager, GaugeDotnet.Devices

### Community 20 - "Linux Storage"
Cohesion: 0.20
Nodes (5): IStorageProvider, LinuxStorageProvider, RG35XX.Libraries, RG35XX.Libraries, StorageProvider

### Community 21 - "Segment Display"
Cohesion: 0.28
Nodes (3): GaugeDotnet.Gauges.Components, SegmentDisplay, SKMaskFilter

### Community 22 - "BLE Types & Keep-Alive"
Cohesion: 0.22
Nodes (6): bool, GaugeDotnet.Devices, RaceChronoIds, Guid, ScreenKeepAlive, string

### Community 27 - "Bar Gauge"
Cohesion: 0.32
Nodes (4): BarGauge, GaugeDotnet.Gauges, SKRectExtensions, SKBitmap

### Community 34 - "Circular Gauge"
Cohesion: 0.33
Nodes (4): CircularGauge, GaugeDotnet.Gauges, SKCanvas, SKRect

### Community 35 - "WSL Launch Config"
Cohesion: 0.29
Nodes (6): commandName, profiles, GaugeDotnet, WSL, commandName, distributionName

### Community 40 - "System Env Variables"
Cohesion: 0.33
Nodes (5): DBUS_SYSTEM_BUS_ADDRESS, DOTNET_SYSTEM_GLOBALIZATION_INVARIANT, LANG, LC_ALL, PATH

### Community 41 - "Font Helper"
Cohesion: 0.33
Nodes (3): FontHelper, GaugeDotnet.Rendering, SKTypeface

## Knowledge Gaps
- **149 isolated node(s):** `LANG`, `LC_ALL`, `PATH`, `DOTNET_SYSTEM_GLOBALIZATION_INVARIANT`, `DBUS_SYSTEM_BUS_ADDRESS` (+144 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **70 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `bool` connect `BLE Types & Keep-Alive` to `Gauge Designer UI`, `BLE Device Connection`, `Circular Gauge`, `Core App & Input`, `Config Editor Screen`, `Built-in Gauges`, `Segment Display`, `Bar Gauge`, `MinMax Gauge`?**
  _High betweenness centrality (0.064) - this node is a cross-community bridge._
- **Why does `MainWindow` connect `Gauge Designer UI` to `Configuration & JSON`, `Data Source & Expression`, `Custom & Histogram Gauge`, `Grid Gauge & Config`, `BLE Types & Keep-Alive`?**
  _High betweenness centrality (0.058) - this node is a cross-community bridge._
- **Why does `int` connect `Grid Gauge & Config` to `Gauge Designer UI`, `Circular Gauge`, `Configuration & JSON`, `Game Loop & Config`, `Config Editor Screen`, `Data Source & Expression`, `Render Context & FPS`, `Custom & Histogram Gauge`, `Segment Display`, `Bar Gauge`, `MinMax Gauge`?**
  _High betweenness centrality (0.057) - this node is a cross-community bridge._
- **What connects `LANG`, `LC_ALL`, `PATH` to the rest of the system?**
  _150 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `BLE Device Connection` be split into smaller, more focused modules?**
  _Cohesion score 0.06463414634146342 - nodes in this community are weakly interconnected._
- **Should `Core App & Input` be split into smaller, more focused modules?**
  _Cohesion score 0.05128205128205128 - nodes in this community are weakly interconnected._
- **Should `Gauge Architecture` be split into smaller, more focused modules?**
  _Cohesion score 0.07692307692307693 - nodes in this community are weakly interconnected._