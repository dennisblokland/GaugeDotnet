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
- [x] Add `LabelValueElement` to `GaugeElement.cs`
- [x] Add `DrawLabelValue` to `ElementRenderer.cs`
- [x] Wire up Designer

### `PeakMarkerElement` ("peak")
Tracks rolling peak, draws short tick at peak angle on arc.  
**Properties**: Radius, StartAngleDeg, SweepAngleDeg, marker width/length/color, decay timeout (0 = hold forever).  
- [x] Add `PeakMarkerElement` to `GaugeElement.cs`
- [x] Add `DrawPeakMarker` + peak tracking to `ElementRenderer.cs`
- [x] Wire up Designer


---

## Phase 2 — Element Enhancements

### Opacity on base `GaugeElement`
Move `Opacity` (byte) from `ImageElement` up to the base class. Renderer applies universally.  
- [x] Add `Opacity` to `GaugeElement` base, remove from `ImageElement`
- [x] Apply opacity via `SaveLayer` in `DrawElement` (covers all element types)
- [x] Add `Opacity` slider to common props section in Designer

### Conditional color — Arc + LinearBar
Threshold-based color switching. Arc/bar turns yellow/red when value exceeds thresholds.  
**Properties**: `WarnThreshold`, `WarnColor`, `DangerThreshold`, `DangerColor` on `ArcElement` + `LinearBarElement`.  
- [x] Add threshold props to `ArcElement` and `LinearBarElement`
- [x] Update `DrawArc` and `DrawLinearBar` to pick color based on value
- [x] Wire up Designer property panels

### `ArcElement` anti-clockwise flag
**Properties**: `AntiClockwise` (bool). Negates sweep direction.  
- [ ] Add `AntiClockwise` to `ArcElement`
- [ ] Flip sweep sign in `DrawArc`
- [ ] Add checkbox in Designer

### `TextElement` background box
**Properties**: `ShowBox` (bool), `BoxColor` (hex), `BoxPadding` (float), `BoxCornerRadius` (float).  
- [x] Add props to `TextElement`
- [x] Draw filled rounded rect behind text in `DrawText`
- [x] Wire up Designer

---

## Phase 3 — Designer UX

### Keyboard nudge
Arrow keys move selected element ±1px; Shift+Arrow ±10px.  
- [x] Add 4 cases to existing `KeyDown` handler in `MainWindow.axaml.cs`

### Undo / Redo
Stack of JSON snapshots of `CustomGaugeDefinition`. Ctrl+Z / Ctrl+Y. Cap at 50 states.  
- [x] Add `_undoStack` / `_redoStack` (`List<string>`, capped at 50) to ViewModel
- [x] Push snapshot before: add, delete, duplicate, move, drag start
- [x] Handle Ctrl+Z / Ctrl+Y in `KeyDown`
- [x] Undo/Redo toolbar buttons with enabled state

### Grid snap
Toggle (G key or toolbar checkbox). Snaps X/Y during drag and property nudge.  
**Default grid**: 10px.  
- [x] Add `_gridSnapEnabled` + `GridSize` const to Designer state
- [x] Snap in `Canvas_PointerMoved` and arrow key nudge
- [x] `ToggleButton` in toolbar; G key toggles

### Multi-select + group move
Shift+click adds to selection. Drag moves all. Delete removes all.  
- [ ] Add `_selection: HashSet<GaugeElement>` to ViewModel
- [ ] Update hit-test, drag, delete, property panel logic

---

## Phase 4 — Data / Logic

### Calculated channels
Virtual data sources defined as simple expressions (`"Rpm / 1000"`, `"OilTemp - Iat"`).  
Evaluated each frame via `ExpressionEvaluator`; appear in DataSource dropdowns.  
- [x] Add `CalculatedChannel` class (`Name`, `Expression`) + `CalculatedChannels` list to `CustomGaugeDefinition`
- [x] `ExpressionEvaluator` — recursive descent (+/-/*/÷/parens/identifiers)
- [x] Evaluate in `ElementRenderer.Render()`, merged into local value dict
- [x] Channel names included in Designer DataSource dropdowns
- [x] Add/remove/edit channels in Canvas Properties panel

### Conditional element visibility
`GaugeElement` base gets `UseVisibility`, `VisibilitySource`, `VisibleAbove`, `VisibleBelow`.  
Element skips draw if condition fails.  
- [x] Add props to `GaugeElement` base
- [x] Skip `DrawElement` when `value < VisibleAbove || value > VisibleBelow`
- [x] Add props to Designer property panel (common section)

---

## Priority Order

| # | Item | Effort | Impact | Status |
|---|------|--------|--------|--------|
| 1 | ZoneArcElement | M | High | [X] |
| 2 | GraphElement | M | High | [X] |
| 3 | Keyboard nudge | XS | High | [X] |
| 4 | Opacity on base element | XS | Med | [x] |
| 5 | LabelValueElement | S | High | [x] |
| 6 | Conditional color (arc/bar) | S | High | [x] |
| 8 | Undo/Redo | M | High | [x] |
| 9 | PeakMarkerElement | M | Med | [x] |
| 10 | TextElement box | S | Med | [x] |
| 12 | Grid snap | M | Med | [x] |
| 13 | Calculated channels | L | Med | [x] |
| 14 | Conditional visibility | M | Med | [x] |
| 15 | ArcElement anti-clockwise | XS | Low | [ ] |
| 16 | Multi-select + group move | L | Med | [ ] |
