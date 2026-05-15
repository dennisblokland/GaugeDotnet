using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Text.Json.Serialization;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Layout;
using Avalonia.Media.Imaging;
using Avalonia.Platform.Storage;
using GaugeDotnet.Configuration;
using GaugeDotnet.Gauges.Custom;
using DesignerRenderer = GaugeDotnet.Designer.Rendering.ElementRenderer;
using SkiaSharp;

namespace GaugeDotnet.Designer;

public partial class MainWindow : Window
{
	private const int CanvasWidth = 640;
	private const int CanvasHeight = 480;

	private static readonly JsonSerializerOptions JsonOptions = new()
	{
		WriteIndented = true,
		DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
	};

	private readonly GaugeDesignerViewModel _vm = new();

	// Drag state
	private bool _isDragging;
	private Point _dragStartMouse;
	private float _dragStartX;
	private float _dragStartY;

	private bool _suppressPropertyEvents;
	private bool _suppressListEvents;

	public MainWindow()
	{
		InitializeComponent();
		WireEvents();
		RefreshElementList();
		UpdateTestValueSliders();
		Redraw();
	}

	// ──────────────────── Event Wiring ────────────────────

	private void WireEvents()
	{
		// Toolbox
		AddArcBtn.Click += (_, _) => AddElement(new ArcElement());
		AddNeedleBtn.Click += (_, _) => AddElement(new NeedleElement());
		AddTicksBtn.Click += (_, _) => AddElement(new TickRingElement());
		AddTextBtn.Click += (_, _) => AddElement(new TextElement());
		AddValueBtn.Click += (_, _) => AddElement(new ValueDisplayElement());
		AddCircleBtn.Click += (_, _) => AddElement(new CircleElement());
		AddRectBtn.Click += (_, _) => AddElement(new RectangleElement());
		AddLineBtn.Click += (_, _) => AddElement(new LineElement());
		AddBarBtn.Click += (_, _) => AddElement(new LinearBarElement());
		AddWarningBtn.Click += (_, _) => AddElement(new WarningIndicatorElement());
		AddImageBtn.Click += (_, _) => AddElement(new ImageElement());
		AddZoneArcBtn.Click += (_, _) => AddElement(new ZoneArcElement());
		AddGraphBtn.Click += (_, _) => AddElement(new GraphElement());
		AddLabelValueBtn.Click += (_, _) => AddElement(new LabelValueElement());

		DuplicateBtn.Click += (_, _) => DuplicateElement();
		DeleteBtn.Click += (_, _) => DeleteElement();

		NewBtn.Click += (_, _) => OnNewClick();
		SaveBtn.Click += OnSaveClick;
		LoadBtn.Click += OnLoadClick;

		// Element list — index 0 is the sentinel "[Canvas] Background" entry
		ElementList.SelectionChanged += (_, _) =>
		{
			if (_suppressListEvents) return;
			int idx = ElementList.SelectedIndex;
			if (idx == 0)
			{
				SelectElement(null);
				return;
			}
			int elIdx = idx - 1;
			GaugeElement? element = elIdx >= 0 && elIdx < _vm.Definition.Elements.Count
				? _vm.Definition.Elements[elIdx]
				: null;
			SelectElement(element);
		};

		MoveUpBtn.Click += (_, _) => MoveElement(-1);
		MoveDownBtn.Click += (_, _) => MoveElement(1);

		// Canvas pointer events
		CanvasImage.PointerPressed += Canvas_PointerPressed;
		CanvasImage.PointerMoved += Canvas_PointerMoved;
		CanvasImage.PointerReleased += Canvas_PointerReleased;

		// Keyboard
		KeyDown += (_, e) =>
		{
			if (e.Key == Key.Delete && _vm.SelectedElement != null)
			{
				DeleteElement();
				e.Handled = true;
				return;
			}

			if (_vm.SelectedElement != null)
			{
				float step = e.KeyModifiers.HasFlag(KeyModifiers.Shift) ? 10f : 1f;
				switch (e.Key)
				{
					case Key.Up:    _vm.SelectedElement.Y -= step; break;
					case Key.Down:  _vm.SelectedElement.Y += step; break;
					case Key.Left:  _vm.SelectedElement.X -= step; break;
					case Key.Right: _vm.SelectedElement.X += step; break;
					default: return;
				}
				ShowProperties(_vm.SelectedElement);
				Redraw();
				e.Handled = true;
			}
		};
	}

	// ──────────────────── Element Management (delegates to ViewModel) ────────────────────

	private void AddElement(GaugeElement element)
	{
		_vm.AddElement(element);
		RefreshElementList();
		UpdateTestValueSliders();
		ShowProperties(_vm.SelectedElement!);
		Redraw();
	}

	private void DeleteElement()
	{
		_vm.DeleteSelected();
		RefreshElementList();
		UpdateTestValueSliders();
		ClearProperties();
		Redraw();
	}

	private void DuplicateElement()
	{
		GaugeElement? copy = _vm.Duplicate();
		if (copy == null) return;
		RefreshElementList();
		ShowProperties(copy);
		Redraw();
	}

	private void MoveElement(int direction)
	{
		_vm.MoveElement(direction);
		RefreshElementList();
		Redraw();
	}

	private void SelectElement(GaugeElement? element)
	{
		_vm.SelectElement(element);
		DeleteBtn.IsEnabled = element != null;
		DuplicateBtn.IsEnabled = element != null;

		_suppressListEvents = true;
		ElementList.SelectedIndex = element != null ? _vm.Definition.Elements.IndexOf(element) + 1 : 0;
		_suppressListEvents = false;

		if (element != null)
			ShowProperties(element);
		else
			ClearProperties();

		Redraw();
	}

	private void RefreshElementList()
	{
		_suppressListEvents = true;
		int prevIdx = _vm.SelectedElement != null
			? _vm.Definition.Elements.IndexOf(_vm.SelectedElement) + 1
			: 0;
		var items = new List<string> { "[Canvas] Background" };
		items.AddRange(_vm.Definition.Elements.Select(e => $"[{e.TypeLabel}] {e.Name}"));
		ElementList.ItemsSource = items;
		if (prevIdx >= 0 && prevIdx < items.Count)
			ElementList.SelectedIndex = prevIdx;
		_suppressListEvents = false;
	}

	// ──────────────────── Canvas Interaction ────────────────────

	private void Canvas_PointerPressed(object? sender, PointerPressedEventArgs e)
	{
		PointerPoint point = e.GetCurrentPoint(CanvasImage);
		float px = (float)point.Position.X;
		float py = (float)point.Position.Y;

		if (point.Properties.IsLeftButtonPressed)
		{
			GaugeElement? hit = null;
			for (int i = _vm.Definition.Elements.Count - 1; i >= 0; i--)
			{
				if (DesignerRenderer.HitTest(_vm.Definition.Elements[i], px, py))
				{
					hit = _vm.Definition.Elements[i];
					break;
				}
			}

			SelectElement(hit);

			if (hit != null)
			{
				_isDragging = true;
				_dragStartMouse = new Point(px, py);
				_dragStartX = hit.X;
				_dragStartY = hit.Y;
				e.Handled = true;
			}
		}
	}

	private void Canvas_PointerMoved(object? sender, PointerEventArgs e)
	{
		if (!_isDragging || _vm.SelectedElement == null) return;

		PointerPoint point = e.GetCurrentPoint(CanvasImage);
		float dx = (float)point.Position.X - (float)_dragStartMouse.X;
		float dy = (float)point.Position.Y - (float)_dragStartMouse.Y;
		_vm.SelectedElement.X = _dragStartX + dx;
		_vm.SelectedElement.Y = _dragStartY + dy;
		Redraw();
	}

	private void Canvas_PointerReleased(object? sender, PointerReleasedEventArgs e)
	{
		if (_isDragging)
		{
			_isDragging = false;
			if (_vm.SelectedElement != null)
				ShowProperties(_vm.SelectedElement);
		}
	}

	// ──────────────────── Property Panel ────────────────────

	private void ClearProperties()
	{
		_suppressPropertyEvents = true;
		PropertiesPanel.Children.Clear();
		AddHeader("Canvas Properties");
		AddColorProp("Background", _vm.Definition.BackgroundColor, v => _vm.Definition.BackgroundColor = v);
		AddImagePathProp("Background Image", _vm.Definition.BackgroundImage ?? "", v =>
			_vm.Definition.BackgroundImage = string.IsNullOrEmpty(v) ? null : v);
		AddEnumProp("Image Mode", _vm.Definition.BackgroundImageMode,
			v => _vm.Definition.BackgroundImageMode = v);
		AddByteProp("Image Opacity", _vm.Definition.BackgroundImageOpacity,
			v => _vm.Definition.BackgroundImageOpacity = v);
		_suppressPropertyEvents = false;
	}

	private void ShowProperties(GaugeElement element)
	{
		_suppressPropertyEvents = true;
		PropertiesPanel.Children.Clear();

		AddHeader($"{element.TypeLabel} Properties");
		AddTextProp("Name", element.Name, v => { element.Name = v; RefreshElementList(); });
		AddFloatProp("X", element.X, v => element.X = v, 0, CanvasWidth);
		AddFloatProp("Y", element.Y, v => element.Y = v, 0, CanvasHeight);

		bool supportsData = element is ArcElement or NeedleElement or ValueDisplayElement
			or LinearBarElement or WarningIndicatorElement or ZoneArcElement or GraphElement
			or LabelValueElement;
		if (supportsData)
		{
			AddDataSourceProp(element.DataSource, v =>
			{
				element.DataSource = v;
				UpdateTestValueSliders();
			});
		}

		bool hasRange = element is ArcElement or NeedleElement or ValueDisplayElement
			or TickRingElement or LinearBarElement or WarningIndicatorElement
			or ZoneArcElement or GraphElement or LabelValueElement;
		if (hasRange)
		{
			AddFloatProp("Min Value", element.MinValue, v => element.MinValue = v, -10000, 100000);
			AddFloatProp("Max Value", element.MaxValue, v => element.MaxValue = v, -10000, 100000);
		}

		AddByteProp("Opacity", element.Opacity, v => element.Opacity = v);
		AddSeparator();

		switch (element)
		{
			case ArcElement arc:
				AddFloatProp("Radius", arc.Radius, v => arc.Radius = v, 10, 300);
				AddFloatProp("Stroke Width", arc.StrokeWidth, v => arc.StrokeWidth = v, 1, 80);
				AddFloatProp("Start Angle", arc.StartAngleDeg, v => arc.StartAngleDeg = v, 0, 360);
				AddFloatProp("Sweep Angle", arc.SweepAngleDeg, v => arc.SweepAngleDeg = v, 1, 360);
				AddColorProp("Color", arc.Color, v => arc.Color = v);
				AddColorProp("Track Color", arc.TrackColor, v => arc.TrackColor = v);
				AddBoolProp("Show Track", arc.ShowTrack, v => arc.ShowTrack = v);
				AddBoolProp("Dynamic (value-driven)", arc.IsDynamic, v => arc.IsDynamic = v);
				AddSeparator();
				AddBoolProp("Conditional Color", arc.UseConditionalColor, v => arc.UseConditionalColor = v);
				AddFloatProp("Warn Threshold", arc.WarnThreshold, v => arc.WarnThreshold = v, -10000, 100000);
				AddColorProp("Warn Color", arc.WarnColor, v => arc.WarnColor = v);
				AddFloatProp("Danger Threshold", arc.DangerThreshold, v => arc.DangerThreshold = v, -10000, 100000);
				AddColorProp("Danger Color", arc.DangerColor, v => arc.DangerColor = v);
				break;

			case NeedleElement needle:
				AddFloatProp("Length", needle.Length, v => needle.Length = v, 10, 300);
				AddFloatProp("Tail Length", needle.TailLength, v => needle.TailLength = v, 0, 100);
				AddFloatProp("Width", needle.NeedleWidth, v => needle.NeedleWidth = v, 1, 20, 0.5f);
				AddFloatProp("Start Angle", needle.StartAngleDeg, v => needle.StartAngleDeg = v, 0, 360);
				AddFloatProp("Sweep Angle", needle.SweepAngleDeg, v => needle.SweepAngleDeg = v, 1, 360);
				AddColorProp("Color", needle.Color, v => needle.Color = v);
				AddBoolProp("Show Hub", needle.ShowHub, v => needle.ShowHub = v);
				AddFloatProp("Hub Radius", needle.HubRadius, v => needle.HubRadius = v, 2, 30);
				AddColorProp("Hub Color", needle.HubColor, v => needle.HubColor = v);
				AddSeparator();
				AddImagePathProp("Needle Image", needle.ImagePath ?? "", v => needle.ImagePath = string.IsNullOrEmpty(v) ? null : v);
				AddFloatProp("Image Width", needle.ImageWidth, v => needle.ImageWidth = v, 2, 100);
				AddFloatProp("Image Length", needle.ImageLength, v => needle.ImageLength = v, 10, 400);
				break;

			case TextElement text:
				AddTextProp("Text", text.Text, v => text.Text = v);
				AddFloatProp("Font Size", text.FontSize, v => text.FontSize = v, 8, 120);
				AddColorProp("Color", text.Color, v => text.Color = v);
				AddFontProp(text.Font, v => text.Font = v);
				break;

			case ValueDisplayElement val:
				AddFloatProp("Font Size", val.FontSize, v => val.FontSize = v, 8, 120);
				AddColorProp("Color", val.Color, v => val.Color = v);
				AddFontProp(val.Font, v => val.Font = v);
				AddTextProp("Format (e.g. F0, F1)", val.Format, v => val.Format = v);
				AddTextProp("Suffix", val.Suffix, v => val.Suffix = v);
				break;

			case TickRingElement ticks:
				AddFloatProp("Radius", ticks.Radius, v => ticks.Radius = v, 10, 300);
				AddFloatProp("Start Angle", ticks.StartAngleDeg, v => ticks.StartAngleDeg = v, 0, 360);
				AddFloatProp("Sweep Angle", ticks.SweepAngleDeg, v => ticks.SweepAngleDeg = v, 1, 360);
				AddIntProp("Major Ticks", ticks.MajorCount, v => ticks.MajorCount = v, 2, 30);
				AddIntProp("Minor Per Major", ticks.MinorPerMajor, v => ticks.MinorPerMajor = v, 0, 10);
				AddFloatProp("Major Length", ticks.MajorLength, v => ticks.MajorLength = v, 2, 50);
				AddFloatProp("Minor Length", ticks.MinorLength, v => ticks.MinorLength = v, 2, 30);
				AddFloatProp("Major Width", ticks.MajorWidth, v => ticks.MajorWidth = v, 0.5f, 10, 0.5f);
				AddFloatProp("Minor Width", ticks.MinorWidth, v => ticks.MinorWidth = v, 0.5f, 5, 0.5f);
				AddColorProp("Color", ticks.Color, v => ticks.Color = v);
				AddBoolProp("Show Ticks", ticks.ShowTicks, v => ticks.ShowTicks = v);
				AddBoolProp("Ticks Inside", ticks.TicksInside, v => ticks.TicksInside = v);
				AddBoolProp("Show Labels", ticks.ShowLabels, v => ticks.ShowLabels = v);
				AddBoolProp("Radial Labels", ticks.RadialLabels, v => ticks.RadialLabels = v);
				AddFloatProp("Label Font Size", ticks.LabelFontSize, v => ticks.LabelFontSize = v, 8, 30);
				AddColorProp("Label Color", ticks.LabelColor, v => ticks.LabelColor = v);
				AddFloatProp("Label Offset", ticks.LabelOffset, v => ticks.LabelOffset = v, 5, 60);
				break;

			case CircleElement circle:
				AddFloatProp("Radius", circle.Radius, v => circle.Radius = v, 2, 200);
				AddColorProp("Fill Color", circle.FillColor, v => circle.FillColor = v);
				AddColorProp("Stroke Color", circle.StrokeColor, v => circle.StrokeColor = v);
				AddFloatProp("Stroke Width", circle.CircleStrokeWidth, v => circle.CircleStrokeWidth = v, 0, 10, 0.5f);
				break;

			case RectangleElement rect:
				AddFloatProp("Width", rect.Width, v => rect.Width = v, 1, 640);
				AddFloatProp("Height", rect.Height, v => rect.Height = v, 1, 480);
				AddColorProp("Fill Color", rect.FillColor, v => rect.FillColor = v);
				AddColorProp("Stroke Color", rect.StrokeColor, v => rect.StrokeColor = v);
				AddFloatProp("Stroke Width", rect.RectStrokeWidth, v => rect.RectStrokeWidth = v, 0, 10, 0.5f);
				AddFloatProp("Corner Radius", rect.CornerRadius, v => rect.CornerRadius = v, 0, 50);
				break;

			case LineElement line:
				AddFloatProp("End X", line.X2, v => line.X2 = v, 0, 640);
				AddFloatProp("End Y", line.Y2, v => line.Y2 = v, 0, 480);
				AddFloatProp("Width", line.LineWidth, v => line.LineWidth = v, 0.5f, 20, 0.5f);
				AddColorProp("Color", line.Color, v => line.Color = v);
				break;

			case LinearBarElement bar:
				AddFloatProp("Width", bar.Width, v => bar.Width = v, 10, 600);
				AddFloatProp("Height", bar.Height, v => bar.Height = v, 4, 400);
				AddBoolProp("Vertical", bar.IsVertical, v => bar.IsVertical = v);
				AddColorProp("Fill Color", bar.FillColor, v => bar.FillColor = v);
				AddColorProp("Track Color", bar.TrackColor, v => bar.TrackColor = v);
				AddColorProp("Border Color", bar.BorderColor, v => bar.BorderColor = v);
				AddFloatProp("Border Width", bar.BorderWidth, v => bar.BorderWidth = v, 0, 5, 0.5f);
				AddFloatProp("Corner Radius", bar.CornerRadius, v => bar.CornerRadius = v, 0, 20);
				AddSeparator();
				AddBoolProp("Conditional Color", bar.UseConditionalColor, v => bar.UseConditionalColor = v);
				AddFloatProp("Warn Threshold", bar.WarnThreshold, v => bar.WarnThreshold = v, -10000, 100000);
				AddColorProp("Warn Color", bar.WarnColor, v => bar.WarnColor = v);
				AddFloatProp("Danger Threshold", bar.DangerThreshold, v => bar.DangerThreshold = v, -10000, 100000);
				AddColorProp("Danger Color", bar.DangerColor, v => bar.DangerColor = v);
				break;

			case WarningIndicatorElement warn:
				AddFloatProp("Radius", warn.Radius, v => warn.Radius = v, 4, 60);
				AddFloatProp("Threshold", warn.Threshold, v => warn.Threshold = v, -10000, 100000);
				AddBoolProp("Trigger Above", warn.TriggerAbove, v => warn.TriggerAbove = v);
				AddColorProp("Active Color", warn.ActiveColor, v => warn.ActiveColor = v);
				AddColorProp("Inactive Color", warn.InactiveColor, v => warn.InactiveColor = v);
				AddBoolProp("Show Label", warn.ShowLabel, v => warn.ShowLabel = v);
				AddTextProp("Label", warn.Label, v => warn.Label = v);
				AddFloatProp("Label Size", warn.LabelFontSize, v => warn.LabelFontSize = v, 8, 30);
				AddColorProp("Label Color", warn.LabelColor, v => warn.LabelColor = v);
				break;

			case ImageElement img:
				AddImagePathProp("Image Path", img.ImagePath, v => img.ImagePath = v);
				AddFloatProp("Width", img.Width, v => img.Width = v, 1, 1280);
				AddFloatProp("Height", img.Height, v => img.Height = v, 1, 960);
				AddFloatProp("Rotation", img.Rotation, v => img.Rotation = v, -360, 360);
				break;

			case ZoneArcElement zone:
				AddFloatProp("Radius", zone.Radius, v => zone.Radius = v, 10, 300);
				AddFloatProp("Stroke Width", zone.StrokeWidth, v => zone.StrokeWidth = v, 1, 80);
				AddFloatProp("Start Angle", zone.StartAngleDeg, v => zone.StartAngleDeg = v, 0, 360);
				AddFloatProp("Sweep Angle", zone.SweepAngleDeg, v => zone.SweepAngleDeg = v, 1, 360);
				AddSeparator();
				AddColorProp("Zone 1 Color", zone.Zone1Color, v => zone.Zone1Color = v);
				AddBoolProp("Show Zone 2", zone.ShowZone2, v => zone.ShowZone2 = v);
				AddFloatProp("Zone 2 Start (value)", zone.Zone2Start, v => zone.Zone2Start = v, -10000, 100000);
				AddColorProp("Zone 2 Color", zone.Zone2Color, v => zone.Zone2Color = v);
				AddBoolProp("Show Zone 3", zone.ShowZone3, v => zone.ShowZone3 = v);
				AddFloatProp("Zone 3 Start (value)", zone.Zone3Start, v => zone.Zone3Start = v, -10000, 100000);
				AddColorProp("Zone 3 Color", zone.Zone3Color, v => zone.Zone3Color = v);
				AddSeparator();
				AddBoolProp("Show Pointer", zone.ShowPointer, v => zone.ShowPointer = v);
				AddColorProp("Pointer Color", zone.PointerColor, v => zone.PointerColor = v);
				AddFloatProp("Pointer Width", zone.PointerWidth, v => zone.PointerWidth = v, 1, 10, 0.5f);
				break;

			case GraphElement graph:
				AddFloatProp("Width", graph.Width, v => graph.Width = v, 20, 640);
				AddFloatProp("Height", graph.Height, v => graph.Height = v, 20, 480);
				AddIntProp("History Depth", graph.HistoryDepth, v => graph.HistoryDepth = v, 5, 300);
				AddColorProp("Line Color", graph.LineColor, v => graph.LineColor = v);
				AddFloatProp("Line Width", graph.LineWidth, v => graph.LineWidth = v, 0.5f, 10, 0.5f);
				AddBoolProp("Show Fill", graph.ShowFill, v => graph.ShowFill = v);
				AddColorProp("Fill Color", graph.FillColor, v => graph.FillColor = v);
				AddByteProp("Fill Opacity", graph.FillOpacity, v => graph.FillOpacity = v);
				AddColorProp("Background Color", graph.BackColor, v => graph.BackColor = v);
				break;

			case LabelValueElement lv:
				AddTextProp("Label", lv.Label, v => lv.Label = v);
				AddFloatProp("Label Size", lv.LabelFontSize, v => lv.LabelFontSize = v, 8, 60);
				AddColorProp("Label Color", lv.LabelColor, v => lv.LabelColor = v);
				AddFontProp(lv.LabelFont, v => lv.LabelFont = v);
				AddSeparator();
				AddFloatProp("Value Size", lv.ValueFontSize, v => lv.ValueFontSize = v, 12, 120);
				AddColorProp("Value Color", lv.ValueColor, v => lv.ValueColor = v);
				AddFontProp(lv.ValueFont, v => lv.ValueFont = v);
				AddTextProp("Format", lv.ValueFormat, v => lv.ValueFormat = v);
				AddTextProp("Suffix", lv.ValueSuffix, v => lv.ValueSuffix = v);
				AddSeparator();
				AddBoolProp("Show Box", lv.ShowBox, v => lv.ShowBox = v);
				AddColorProp("Box Color", lv.BoxColor, v => lv.BoxColor = v);
				AddFloatProp("Box Padding", lv.BoxPadding, v => lv.BoxPadding = v, 0, 40);
				AddFloatProp("Box Corner Radius", lv.BoxCornerRadius, v => lv.BoxCornerRadius = v, 0, 30);
				break;
		}

		_suppressPropertyEvents = false;
	}

	// ──────────────────── Property Helpers ────────────────────

	private void AddHeader(string text)
	{
		PropertiesPanel.Children.Add(new TextBlock
		{
			Text = text,
			FontWeight = Avalonia.Media.FontWeight.Bold,
			FontSize = 15,
			Margin = new Thickness(0, 4, 0, 2),
		});
	}

	private void AddSeparator()
	{
		PropertiesPanel.Children.Add(new Separator { Margin = new Thickness(0, 6) });
	}

	private void AddTextProp(string label, string value, Action<string> setter)
	{
		PropertiesPanel.Children.Add(new TextBlock { Text = label, Margin = new Thickness(0, 2, 0, 0) });
		TextBox tb = new() { Text = value };
		tb.TextChanged += (_, _) =>
		{
			if (!_suppressPropertyEvents) { setter(tb.Text ?? ""); Redraw(); }
		};
		PropertiesPanel.Children.Add(tb);
	}

	private void AddFloatProp(string label, float value, Action<float> setter,
		float min = -1000, float max = 10000, float step = 1)
	{
		PropertiesPanel.Children.Add(new TextBlock { Text = label, Margin = new Thickness(0, 2, 0, 0) });
		NumericUpDown nud = new()
		{
			Value = (decimal)value,
			Minimum = (decimal)min,
			Maximum = (decimal)max,
			Increment = (decimal)step,
			FormatString = step < 1 ? "F1" : "F0",
		};
		nud.ValueChanged += (_, _) =>
		{
			if (!_suppressPropertyEvents && nud.Value.HasValue) { setter((float)nud.Value.Value); Redraw(); }
		};
		PropertiesPanel.Children.Add(nud);
	}

	private void AddIntProp(string label, int value, Action<int> setter, int min = 0, int max = 100)
	{
		PropertiesPanel.Children.Add(new TextBlock { Text = label, Margin = new Thickness(0, 2, 0, 0) });
		NumericUpDown nud = new()
		{
			Value = value,
			Minimum = min,
			Maximum = max,
			Increment = 1,
			FormatString = "F0",
		};
		nud.ValueChanged += (_, _) =>
		{
			if (!_suppressPropertyEvents && nud.Value.HasValue) { setter((int)nud.Value.Value); Redraw(); }
		};
		PropertiesPanel.Children.Add(nud);
	}

	private void AddColorProp(string label, string value, Action<string> setter)
	{
		PropertiesPanel.Children.Add(new TextBlock { Text = label, Margin = new Thickness(0, 2, 0, 0) });
		TextBox tb = new() { Text = value };
		tb.TextChanged += (_, _) =>
		{
			if (_suppressPropertyEvents) return;
			string hex = tb.Text ?? "#FFFFFF";
			if (SKColor.TryParse(hex, out _)) { setter(hex); Redraw(); }
		};
		PropertiesPanel.Children.Add(tb);
	}

	private void AddBoolProp(string label, bool value, Action<bool> setter)
	{
		CheckBox cb = new() { Content = label, IsChecked = value, Margin = new Thickness(0, 2, 0, 0) };
		cb.IsCheckedChanged += (_, _) =>
		{
			if (!_suppressPropertyEvents) { setter(cb.IsChecked ?? false); Redraw(); }
		};
		PropertiesPanel.Children.Add(cb);
	}

	private void AddEnumProp<T>(string label, T value, Action<T> setter) where T : struct, Enum
	{
		PropertiesPanel.Children.Add(new TextBlock { Text = label, Margin = new Thickness(0, 2, 0, 0) });
		string[] names = Enum.GetNames<T>();
		ComboBox cb = new()
		{
			ItemsSource = names,
			SelectedItem = value.ToString(),
			HorizontalAlignment = HorizontalAlignment.Stretch,
		};
		cb.SelectionChanged += (_, _) =>
		{
			if (!_suppressPropertyEvents && cb.SelectedItem is string s &&
				Enum.TryParse<T>(s, out T parsed)) { setter(parsed); Redraw(); }
		};
		PropertiesPanel.Children.Add(cb);
	}

	private void AddByteProp(string label, byte value, Action<byte> setter)
	{
		PropertiesPanel.Children.Add(new TextBlock { Text = label, Margin = new Thickness(0, 2, 0, 0) });
		Slider sl = new() { Minimum = 0, Maximum = 255, Value = value, TickFrequency = 1 };
		TextBlock display = new() { Text = value.ToString() };
		sl.PropertyChanged += (_, e) =>
		{
			if (e.Property == Slider.ValueProperty && !_suppressPropertyEvents)
			{
				byte b = (byte)sl.Value;
				display.Text = b.ToString();
				setter(b);
				Redraw();
			}
		};
		PropertiesPanel.Children.Add(sl);
		PropertiesPanel.Children.Add(display);
	}

	private void AddDataSourceProp(string? value, Action<string?> setter)
	{
		PropertiesPanel.Children.Add(new TextBlock
		{
			Text = "Data Source",
			FontWeight = Avalonia.Media.FontWeight.SemiBold,
			Margin = new Thickness(0, 4, 0, 0),
		});
		List<string> items = ["(none)"];
		items.AddRange(DataSourceMapper.DataSourceNames);
		ComboBox cb = new()
		{
			ItemsSource = items,
			SelectedItem = value ?? "(none)",
			HorizontalAlignment = HorizontalAlignment.Stretch,
		};
		cb.SelectionChanged += (_, _) =>
		{
			if (_suppressPropertyEvents) return;
			string sel = cb.SelectedItem?.ToString() ?? "(none)";
			setter(sel == "(none)" ? null : sel);
			Redraw();
		};
		PropertiesPanel.Children.Add(cb);
	}

	private void AddFontProp(string value, Action<string> setter)
	{
		PropertiesPanel.Children.Add(new TextBlock { Text = "Font", Margin = new Thickness(0, 2, 0, 0) });
		string[] fonts = ["Race Sport", "DSEG14 Classic", "DSEG7 Classic"];
		ComboBox cb = new()
		{
			ItemsSource = fonts,
			SelectedItem = value,
			HorizontalAlignment = HorizontalAlignment.Stretch,
		};
		cb.SelectionChanged += (_, _) =>
		{
			if (!_suppressPropertyEvents && cb.SelectedItem is string font) { setter(font); Redraw(); }
		};
		PropertiesPanel.Children.Add(cb);
	}

	private void AddImagePathProp(string label, string value, Action<string> setter)
	{
		PropertiesPanel.Children.Add(new TextBlock { Text = label, Margin = new Thickness(0, 2, 0, 0) });
		StackPanel row = new() { Orientation = Orientation.Horizontal, Spacing = 4 };
		TextBox tb = new() { Text = value, Width = 160 };
		Button browseBtn = new() { Content = "Browse", Padding = new Thickness(8, 2) };
		browseBtn.Click += async (_, _) =>
		{
			TopLevel? topLevel = GetTopLevel(this);
			if (topLevel == null) return;

			IReadOnlyList<IStorageFile> files = await topLevel.StorageProvider.OpenFilePickerAsync(
				new FilePickerOpenOptions
				{
					Title = $"Select {label}",
					AllowMultiple = false,
					FileTypeFilter = [new FilePickerFileType("Images") { Patterns = ["*.png", "*.jpg", "*.jpeg", "*.webp", "*.bmp"] }],
				});

			if (files.Count > 0)
			{
				string path = files[0].Path.LocalPath;
				tb.Text = path;
				GaugeDotnet.Gauges.Custom.ElementRenderer.ClearImageCache();
				GaugeDotnet.Gauges.Custom.ElementRenderer.SetBaseDirectory(Path.GetDirectoryName(path));
				setter(path);
				Redraw();
			}
		};
		tb.TextChanged += (_, _) =>
		{
			if (!_suppressPropertyEvents)
			{
				GaugeDotnet.Gauges.Custom.ElementRenderer.ClearImageCache();
				setter(tb.Text ?? "");
				Redraw();
			}
		};
		row.Children.Add(tb);
		row.Children.Add(browseBtn);
		PropertiesPanel.Children.Add(row);
	}

	// ──────────────────── Test Values ────────────────────

	private readonly Dictionary<string, float> _testValues = new();

	private void UpdateTestValueSliders()
	{
		TestValuesPanel.Children.Clear();
		_testValues.Clear();
		HashSet<string> seen = new();

		foreach (GaugeElement element in _vm.Definition.Elements)
		{
			if (string.IsNullOrEmpty(element.DataSource) || !seen.Add(element.DataSource))
				continue;

			string source = element.DataSource;
			float minVal = _vm.Definition.Elements.Where(e => e.DataSource == source).Min(e => e.MinValue);
			float maxVal = _vm.Definition.Elements.Where(e => e.DataSource == source).Max(e => e.MaxValue);

			_testValues[source] = minVal;

			TextBlock lbl = new() { Text = $"{source}: {minVal:F0}" };
			Slider slider = new() { Minimum = minVal, Maximum = maxVal, Value = minVal };
			slider.PropertyChanged += (_, e) =>
			{
				if (e.Property == Slider.ValueProperty)
				{
					lbl.Text = $"{source}: {slider.Value:F1}";
					_testValues[source] = (float)slider.Value;
					Redraw();
				}
			};
			TestValuesPanel.Children.Add(lbl);
			TestValuesPanel.Children.Add(slider);
		}

		if (TestValuesPanel.Children.Count == 0)
		{
			TestValuesPanel.Children.Add(new TextBlock
			{
				Text = "No data-bound elements",
				Foreground = Avalonia.Media.Brushes.Gray,
				FontStyle = Avalonia.Media.FontStyle.Italic,
			});
		}
	}

	// ──────────────────── Rendering ────────────────────

	private void Redraw()
	{
		try
		{
			using SKBitmap bitmap = new(CanvasWidth, CanvasHeight, SKColorType.Rgba8888, SKAlphaType.Premul);
			using SKCanvas canvas = new(bitmap);
			DesignerRenderer.DrawAll(canvas, _vm.Definition, _testValues, _vm.SelectedElement);

			using SKImage image = SKImage.FromBitmap(bitmap);
			using SKData data = image.Encode(SKEncodedImageFormat.Png, 100);
			using MemoryStream stream = new(data.ToArray());
			CanvasImage.Source = new Bitmap(stream);
		}
		catch (Exception ex)
		{
			Console.WriteLine($"Render error: {ex.Message}");
		}
	}

	// ──────────────────── Save / Load / New ────────────────────

	private async void OnSaveClick(object? sender, Avalonia.Interactivity.RoutedEventArgs e)
	{
		TopLevel? topLevel = GetTopLevel(this);
		if (topLevel == null) return;

		IStorageFile? file = await topLevel.StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions
		{
			Title = "Save Gauge Definition",
			DefaultExtension = "json",
			SuggestedFileName = "custom-gauge.json",
			FileTypeChoices = [new FilePickerFileType("JSON") { Patterns = ["*.json"] }],
		});

		if (file != null)
		{
			string saveDir = Path.GetDirectoryName(file.Path.LocalPath) ?? ".";
			string imagesDir = Path.Combine(saveDir, "images");
			CustomGaugeDefinition exportDef = CopyImagesAndRewritePaths(_vm.Definition, saveDir, imagesDir);
			string json = JsonSerializer.Serialize(exportDef, JsonOptions);
			await File.WriteAllTextAsync(file.Path.LocalPath, json);

			// Adopt the exported paths so continued editing uses saved relative paths
			int selectedIdx = _vm.SelectedElement != null
				? _vm.Definition.Elements.IndexOf(_vm.SelectedElement)
				: -1;
			_vm.Load(exportDef);
			GaugeDotnet.Gauges.Custom.ElementRenderer.ClearImageCache();
			GaugeDotnet.Gauges.Custom.ElementRenderer.SetBaseDirectory(saveDir);

			RefreshElementList();
			GaugeElement? restored = selectedIdx >= 0 && selectedIdx < _vm.Definition.Elements.Count
				? _vm.Definition.Elements[selectedIdx]
				: null;
			_vm.SelectElement(restored);
			if (restored != null) ShowProperties(restored); else ClearProperties();
			Redraw();
		}
	}

	private static CustomGaugeDefinition CopyImagesAndRewritePaths(
		CustomGaugeDefinition definition, string saveDir, string imagesDir)
	{
		string tmp = JsonSerializer.Serialize(definition, JsonOptions);
		CustomGaugeDefinition export = JsonSerializer.Deserialize<CustomGaugeDefinition>(tmp, JsonOptions)
			?? new CustomGaugeDefinition();

		bool dirCreated = false;
		int imageIndex = 0;

		export.BackgroundImage = SaveAndRelativize(export.BackgroundImage, imagesDir, ref dirCreated, ref imageIndex);

		foreach (GaugeElement element in export.Elements)
		{
			if (element is ImageElement img)
				img.ImagePath = SaveAndRelativize(img.ImagePath, imagesDir, ref dirCreated, ref imageIndex) ?? "";
			else if (element is NeedleElement needle)
				needle.ImagePath = SaveAndRelativize(needle.ImagePath, imagesDir, ref dirCreated, ref imageIndex);
		}

		return export;
	}

	private static string? SaveAndRelativize(string? path, string imagesDir, ref bool dirCreated, ref int imageIndex)
	{
		if (string.IsNullOrEmpty(path)) return path;
		if (!dirCreated) { Directory.CreateDirectory(imagesDir); dirCreated = true; }
		imageIndex++;
		string destPath = Path.Combine(imagesDir, $"image_{imageIndex}.png");
		GaugeDotnet.Gauges.Custom.ElementRenderer.SaveImageFromCache(path, destPath);
		return Path.Combine("images", $"image_{imageIndex}.png");
	}

	private async void OnLoadClick(object? sender, Avalonia.Interactivity.RoutedEventArgs e)
	{
		TopLevel? topLevel = GetTopLevel(this);
		if (topLevel == null) return;

		IReadOnlyList<IStorageFile> files = await topLevel.StorageProvider.OpenFilePickerAsync(
			new FilePickerOpenOptions
			{
				Title = "Load Gauge Definition",
				AllowMultiple = false,
				FileTypeFilter = [new FilePickerFileType("JSON") { Patterns = ["*.json"] }],
			});

		if (files.Count == 0) return;

		string json = await File.ReadAllTextAsync(files[0].Path.LocalPath);
		CustomGaugeDefinition? loaded = JsonSerializer.Deserialize<CustomGaugeDefinition>(json, JsonOptions);
		if (loaded == null) return;

		string? directory = Path.GetDirectoryName(files[0].Path.LocalPath);
		GaugeDotnet.Gauges.Custom.ElementRenderer.ClearImageCache();
		GaugeDotnet.Gauges.Custom.ElementRenderer.SetBaseDirectory(directory);

		_vm.Load(loaded);
		RefreshElementList();
		UpdateTestValueSliders();
		ClearProperties();
		Redraw();
	}

	private void OnNewClick()
	{
		_vm.New();
		RefreshElementList();
		UpdateTestValueSliders();
		ClearProperties();
		Redraw();
	}
}
