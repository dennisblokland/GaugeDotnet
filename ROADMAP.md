# GaugeDotnet Roadmap

## Phase 1 — New Elements

### `ZoneArcElement` ("zonearc")
Arc with up to 3 colored bands (green/yellow/red zones).  
**Properties**: up to 3 zone thresholds + colors, start/sweep angles, radius, stroke width.  
**Why**: Essential visual for tach, boost, oil pressure gauges.  
- [x] Add `ZoneArcElement` to `GaugeElement.cs`
- [x] Add `DrawZoneArc` to `ElementRenderer.cs`
- [x] Wire up Designer property panel + toolbar button

### `GraphElement` ("graph")
Mini time-series plot as an element. Rolls a per-source float buffer each frame.  
**Properties**: Width, Height, history depth, line color, fill color, fill opacity.  
**Why**: Show RPM/boost history inline on a custom gauge face.  
- [x] Add `GraphElement` to `GaugeElement.cs`
- [x] Add `DrawGraph` + rolling buffer logic to `ElementRenderer.cs`
- [x] Wire up Designer

### `LabelValueElement` ("labelvalue")
Compound element: label text above, large value below, optional background box.  
**Properties**: label text, label font size, value font size, colors, box bg color, padding, format string, suffix.  
**Why**: Replaces 2–3 manually positioned elements. Very common dashboard pattern.  
- [ ] Add `LabelValueElement` to `GaugeElement.cs`
- [ ] Add `DrawLabelValue` to `ElementRenderer.cs`
- [ ] Wire up Designer

### `PeakMarkerElement` ("peak")
Tracks rolling peak, draws short tick at peak angle on arc.  
**Properties**: Radius, StartAngleDeg, SweepAngleDeg, marker width/length/color, decay timeout (0 = hold forever).  
- [ ] Add `PeakMarkerElement` to `GaugeElement.cs`
- [ ] Add `DrawPeakMarker` + peak tracking to `ElementRenderer.cs`
- [ ] Wire up Designer


---

## Phase 2 — Element Enhancements

### Opacity on base `GaugeElement`
Move `Opacity` (byte) from `ImageElement` up to the base class. Renderer applies universally.  
- [ ] Add `Opacity` to `GaugeElement` base, remove from `ImageElement`
- [ ] Apply opacity in each `Draw*` method via `_paint.Color.WithAlpha()`
- [ ] Add `AddByteProp("Opacity", ...)` to all element property panels in Designer

### Conditional color — Arc + LinearBar
Threshold-based color switching. Arc/bar turns yellow/red when value exceeds thresholds.  
**Properties**: `WarnThreshold`, `WarnColor`, `DangerThreshold`, `DangerColor` on `ArcElement` + `LinearBarElement`.  
- [ ] Add threshold props to `ArcElement` and `LinearBarElement`
- [ ] Update `DrawArc` and `DrawLinearBar` to pick color based on value
- [ ] Wire up Designer property panels

### `ArcElement` anti-clockwise flag
**Properties**: `AntiClockwise` (bool). Negates sweep direction.  
- [ ] Add `AntiClockwise` to `ArcElement`
- [ ] Flip sweep sign in `DrawArc`
- [ ] Add checkbox in Designer

### `TextElement` background box
**Properties**: `ShowBox` (bool), `BoxColor` (hex), `BoxPadding` (float), `BoxCornerRadius` (float).  
- [ ] Add props to `TextElement`
- [ ] Draw filled rounded rect behind text in `DrawText`
- [ ] Wire up Designer

---

## Phase 3 — Designer UX

### Keyboard nudge
Arrow keys move selected element ±1px; Shift+Arrow ±10px.  
- [x] Add 4 cases to existing `KeyDown` handler in `MainWindow.axaml.cs`

### Undo / Redo
Stack of JSON snapshots of `CustomGaugeDefinition`. Ctrl+Z / Ctrl+Y. Cap at 50 states.  
- [ ] Add `_undoStack` / `_redoStack` (Stack<string>) to ViewModel
- [ ] Push snapshot before every mutating operation
- [ ] Handle Ctrl+Z / Ctrl+Y in `KeyDown`
- [ ] Update toolbar button states

### Grid snap
Toggle (G key or toolbar checkbox). Snaps X/Y during drag and property nudge.  
**Default grid**: 10px.  
- [ ] Add `GridSnapEnabled` + `GridSize` to Designer state
- [ ] Snap in `Canvas_PointerMoved` and `AddFloatProp` nudge
- [ ] Toggle button in toolbar

### Multi-select + group move
Shift+click adds to selection. Drag moves all. Delete removes all.  
- [ ] Add `_selection: HashSet<GaugeElement>` to ViewModel
- [ ] Update hit-test, drag, delete, property panel logic

---

## Phase 4 — Data / Logic

### Calculated channels
Virtual data sources defined as simple expressions (`"Rpm / 1000"`, `"OilTemp - Iat"`).  
Parsed at load into compiled lambdas; appear in DataSource dropdowns like real channels.  
- [ ] Add `CalculatedChannel` class to config (`Name`, `Expression`)
- [ ] Extend `DataSourceMapper` to evaluate expressions
- [ ] Surface channel definitions in Designer and `gauges.json`

### Conditional element visibility
`GaugeElement` base gets `VisibilitySource`, `VisibleAbove`, `VisibleBelow`.  
Element skips draw if condition fails.  
- [ ] Add props to `GaugeElement` base
- [ ] Skip `DrawElement` when condition not met
- [ ] Add props to Designer property panel

---

## Priority Order

| # | Item | Effort | Impact | Status |
|---|------|--------|--------|--------|
| 1 | ZoneArcElement | M | High | [ ] |
| 2 | GraphElement | M | High | [ ] |
| 3 | Keyboard nudge | XS | High | [ ] |
| 4 | Opacity on base element | XS | Med | [ ] |
| 5 | LabelValueElement | S | High | [ ] |
| 6 | Conditional color (arc/bar) | S | High | [ ] |
| 8 | Undo/Redo | M | High | [ ] |
| 9 | PeakMarkerElement | M | Med | [ ] |
| 10 | TextElement box | S | Med | [ ] |
| 12 | Grid snap | M | Med | [ ] |
| 13 | Calculated channels | L | Med | [ ] |
| 14 | Conditional visibility | M | Med | [ ] |
| 15 | ArcElement anti-clockwise | XS | Low | [ ] |
| 16 | Multi-select + group move | L | Med | [ ] |
