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

	private CustomGaugeDefinition _definition;
	private GaugeElement? _selectedElement;
	private readonly Dictionary<string, float> _testValues = new();
	private int _elementCounter;

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
		_definition = CreateDefaultGauge();
		_elementCounter = _definition.Elements.Count;
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

		DuplicateBtn.Click += (_, _) => DuplicateSelectedElement();
		DeleteBtn.Click += (_, _) => DeleteSelectedElement();

		NewBtn.Click += (_, _) => OnNewClick();
		SaveBtn.Click += OnSaveClick;
		LoadBtn.Click += OnLoadClick;

		// Element list
		ElementList.SelectionChanged += (_, _) =>
		{
			if (_suppressListEvents) return;
			int idx = ElementList.SelectedIndex;
			if (idx >= 0 && idx < _definition.Elements.Count)
			{
				SelectElement(_definition.Elements[idx]);
			}
			else
			{
				SelectElement(null);
			}
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
			if (e.Key == Key.Delete && _selectedElement != null)
			{
				DeleteSelectedElement();
				e.Handled = true;
			}
		};
	}

	// ──────────────────── Element Management ────────────────────

	private void AddElement(GaugeElement element)
	{
		_elementCounter++;
		if (string.IsNullOrEmpty(element.Name))
		{
			element.Name = $"{element.TypeLabel} {_elementCounter}";
		}
		_definition.Elements.Add(element);
		RefreshElementList();
		SelectElement(element);
		UpdateTestValueSliders();
		Redraw();
	}

	private void DeleteSelectedElement()
	{
		if (_selectedElement == null) return;
		_definition.Elements.Remove(_selectedElement);
		SelectElement(null);
		RefreshElementList();
		UpdateTestValueSliders();
		Redraw();
	}

	private void DuplicateSelectedElement()
	{
		if (_selectedElement == null) return;
		string json = JsonSerializer.Serialize<GaugeElement>(_selectedElement, JsonOptions);
		GaugeElement? copy = JsonSerializer.Deserialize<GaugeElement>(json, JsonOptions);
		if (copy == null) return;

		_elementCounter++;
		copy.Id = Guid.NewGuid().ToString("N")[..8];
		copy.Name = $"{copy.Name} copy";
		copy.X += 20;
		copy.Y += 20;
		_definition.Elements.Add(copy);
		RefreshElementList();
		SelectElement(copy);
		Redraw();
	}

	private void MoveElement(int direction)
	{
		if (_selectedElement == null) return;
		int idx = _definition.Elements.IndexOf(_selectedElement);
		int newIdx = idx + direction;
		if (newIdx < 0 || newIdx >= _definition.Elements.Count) return;

		_definition.Elements.RemoveAt(idx);
		_definition.Elements.Insert(newIdx, _selectedElement);
		RefreshElementList();
		Redraw();
	}

	private void SelectElement(GaugeElement? element)
	{
		_selectedElement = element;
		DeleteBtn.IsEnabled = element != null;
		DuplicateBtn.IsEnabled = element != null;

		if (element != null)
		{
			_suppressListEvents = true;
			ElementList.SelectedIndex = _definition.Elements.IndexOf(element);
			_suppressListEvents = false;
			ShowProperties(element);
		}
		else
		{
			_suppressListEvents = true;
			ElementList.SelectedIndex = -1;
			_suppressListEvents = false;
			ClearProperties();
		}
		Redraw();
	}

	private void RefreshElementList()
	{
		_suppressListEvents = true;
		int prevIdx = _selectedElement != null ? _definition.Elements.IndexOf(_selectedElement) : -1;
		ElementList.ItemsSource = _definition.Elements.Select(e => $"[{e.TypeLabel}] {e.Name}").ToList();
		if (prevIdx >= 0 && prevIdx < _definition.Elements.Count)
		{
			ElementList.SelectedIndex = prevIdx;
		}
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
			// Hit test in reverse order (front-most first)
			GaugeElement? hit = null;
			for (int i = _definition.Elements.Count - 1; i >= 0; i--)
			{
				if (DesignerRenderer.HitTest(_definition.Elements[i], px, py))
				{
					hit = _definition.Elements[i];
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
		if (!_isDragging || _selectedElement == null) return;

		PointerPoint point = e.GetCurrentPoint(CanvasImage);
		float dx = (float)point.Position.X - (float)_dragStartMouse.X;
		float dy = (float)point.Position.Y - (float)_dragStartMouse.Y;
		_selectedElement.X = _dragStartX + dx;
		_selectedElement.Y = _dragStartY + dy;
		Redraw();
	}

	private void Canvas_PointerReleased(object? sender, PointerReleasedEventArgs e)
	{
		if (_isDragging)
		{
			_isDragging = false;
			if (_selectedElement != null)
			{
				ShowProperties(_selectedElement);
			}
		}
	}

	// ──────────────────── Property Panel ────────────────────

	private void ClearProperties()
	{
		_suppressPropertyEvents = true;
		PropertiesPanel.Children.Clear();
		AddHeader("Canvas Properties");
		AddColorProp("Background", _definition.BackgroundColor, v =>
		{
			_definition.BackgroundColor = v;
		});
		AddImagePathProp("Background Image", _definition.BackgroundImage ?? "", v =>
		{
			_definition.BackgroundImage = string.IsNullOrEmpty(v) ? null : v;
		});
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

		// Data binding (only for dynamic element types)
		bool supportsData = element is ArcElement or NeedleElement or ValueDisplayElement
			or LinearBarElement or WarningIndicatorElement;
		if (supportsData)
		{
			AddDataSourceProp(element.DataSource, v =>
			{
				element.DataSource = v;
				UpdateTestValueSliders();
			});
		}

		bool hasRange = element is ArcElement or NeedleElement or ValueDisplayElement
			or TickRingElement or LinearBarElement or WarningIndicatorElement;
		if (hasRange)
		{
			AddFloatProp("Min Value", element.MinValue, v => element.MinValue = v, -10000, 100000);
			AddFloatProp("Max Value", element.MaxValue, v => element.MaxValue = v, -10000, 100000);
		}

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
				AddIntProp("Opacity", img.Opacity, v => img.Opacity = (byte)v, 0, 255);
				AddFloatProp("Rotation", img.Rotation, v => img.Rotation = v, -360, 360);
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
			if (!_suppressPropertyEvents)
			{
				setter(tb.Text ?? "");
				Redraw();
			}
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
			if (!_suppressPropertyEvents && nud.Value.HasValue)
			{
				setter((float)nud.Value.Value);
				Redraw();
			}
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
			if (!_suppressPropertyEvents && nud.Value.HasValue)
			{
				setter((int)nud.Value.Value);
				Redraw();
			}
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
			if (SKColor.TryParse(hex, out _))
			{
				setter(hex);
				Redraw();
			}
		};
		PropertiesPanel.Children.Add(tb);
	}

	private void AddBoolProp(string label, bool value, Action<bool> setter)
	{
		CheckBox cb = new() { Content = label, IsChecked = value, Margin = new Thickness(0, 2, 0, 0) };
		cb.IsCheckedChanged += (_, _) =>
		{
			if (!_suppressPropertyEvents)
			{
				setter(cb.IsChecked ?? false);
				Redraw();
			}
		};
		PropertiesPanel.Children.Add(cb);
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
			if (!_suppressPropertyEvents && cb.SelectedItem is string font)
			{
				setter(font);
				Redraw();
			}
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

	private void UpdateTestValueSliders()
	{
		TestValuesPanel.Children.Clear();
		_testValues.Clear();
		HashSet<string> seen = new();

		foreach (GaugeElement element in _definition.Elements)
		{
			if (string.IsNullOrEmpty(element.DataSource) || !seen.Add(element.DataSource))
				continue;

			string source = element.DataSource;
			float minVal = _definition.Elements
				.Where(e => e.DataSource == source).Min(e => e.MinValue);
			float maxVal = _definition.Elements
				.Where(e => e.DataSource == source).Max(e => e.MaxValue);

			_testValues[source] = minVal;

			TextBlock label = new() { Text = $"{source}: {minVal:F0}" };
			Slider slider = new()
			{
				Minimum = minVal,
				Maximum = maxVal,
				Value = minVal,
			};
			slider.PropertyChanged += (_, e) =>
			{
				if (e.Property == Slider.ValueProperty)
				{
					label.Text = $"{source}: {slider.Value:F1}";
					_testValues[source] = (float)slider.Value;
					Redraw();
				}
			};
			TestValuesPanel.Children.Add(label);
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
			DesignerRenderer.DrawAll(canvas, _definition, _testValues, _selectedElement);

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
			CustomGaugeDefinition exportDef = CopyImagesAndRewritePaths(_definition, saveDir, imagesDir);
			string json = JsonSerializer.Serialize(exportDef, JsonOptions);
			await File.WriteAllTextAsync(file.Path.LocalPath, json);

			// Adopt the exported paths so continued editing uses the saved relative paths
			int selectedIdx = _selectedElement != null ? _definition.Elements.IndexOf(_selectedElement) : -1;
			_definition = exportDef;
			_selectedElement = selectedIdx >= 0 && selectedIdx < _definition.Elements.Count
				? _definition.Elements[selectedIdx]
				: null;
			GaugeDotnet.Gauges.Custom.ElementRenderer.ClearImageCache();
			GaugeDotnet.Gauges.Custom.ElementRenderer.SetBaseDirectory(saveDir);
			RefreshElementList();
			if (_selectedElement != null)
			{
				ShowProperties(_selectedElement);
			}
			else
			{
				ClearProperties();
			}
			Redraw();
		}
	}

	private static CustomGaugeDefinition CopyImagesAndRewritePaths(
		CustomGaugeDefinition definition, string saveDir, string imagesDir)
	{
		// Deep-copy via JSON round-trip
		string tmp = JsonSerializer.Serialize(definition, JsonOptions);
		CustomGaugeDefinition export = JsonSerializer.Deserialize<CustomGaugeDefinition>(tmp, JsonOptions)
			?? new CustomGaugeDefinition();

		bool dirCreated = false;
		int imageIndex = 0;

		export.BackgroundImage = SaveAndRelativize(export.BackgroundImage, imagesDir, ref dirCreated, ref imageIndex);

		foreach (GaugeElement element in export.Elements)
		{
			if (element is ImageElement img)
			{
				img.ImagePath = SaveAndRelativize(img.ImagePath, imagesDir, ref dirCreated, ref imageIndex) ?? "";
			}
			else if (element is NeedleElement needle)
			{
				needle.ImagePath = SaveAndRelativize(needle.ImagePath, imagesDir, ref dirCreated, ref imageIndex);
			}
		}

		return export;
	}

	private static string? SaveAndRelativize(string? path, string imagesDir, ref bool dirCreated, ref int imageIndex)
	{
		if (string.IsNullOrEmpty(path)) return path;

		if (!dirCreated)
		{
			Directory.CreateDirectory(imagesDir);
			dirCreated = true;
		}

		imageIndex++;
		string fileName = $"image_{imageIndex}.png";
		string destPath = Path.Combine(imagesDir, fileName);

		GaugeDotnet.Gauges.Custom.ElementRenderer.SaveImageFromCache(path, destPath);

		return Path.Combine("images", fileName);
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

		_definition = loaded;
		_selectedElement = null;
		_elementCounter = _definition.Elements.Count;
		RefreshElementList();
		UpdateTestValueSliders();
		ClearProperties();
		Redraw();
	}

	private void OnNewClick()
	{
		_definition = new CustomGaugeDefinition();
		_selectedElement = null;
		_elementCounter = 0;
		RefreshElementList();
		UpdateTestValueSliders();
		ClearProperties();
		Redraw();
	}

	// ──────────────────── Default Gauge ────────────────────

	private static CustomGaugeDefinition CreateDefaultGauge()
	{
		return new CustomGaugeDefinition
		{
			Elements =
			[
				new ArcElement
				{
					Name = "RPM Arc",
					X = 320, Y = 240,
					Radius = 200, StrokeWidth = 30,
					StartAngleDeg = 135, SweepAngleDeg = 270,
					Color = "#00FFFF", TrackColor = "#0A1A1A",
					ShowTrack = true, IsDynamic = true,
					DataSource = "Rpm", MinValue = 0, MaxValue = 8000,
				},
				new TickRingElement
				{
					Name = "RPM Ticks",
					X = 320, Y = 240,
					Radius = 178, StartAngleDeg = 135, SweepAngleDeg = 270,
					MajorCount = 8, MinorPerMajor = 4,
					MajorLength = 15, MinorLength = 8,
					MajorWidth = 2, MinorWidth = 1,
					Color = "#AAAAAA",
					ShowLabels = true, LabelFontSize = 14,
					LabelColor = "#888888", LabelOffset = 22,
					MinValue = 0, MaxValue = 8000,
				},
				new NeedleElement
				{
					Name = "RPM Needle",
					X = 320, Y = 240,
					Length = 170, TailLength = 25, NeedleWidth = 4,
					StartAngleDeg = 135, SweepAngleDeg = 270,
					Color = "#FF3333",
					ShowHub = true, HubRadius = 10, HubColor = "#CCCCCC",
					DataSource = "Rpm", MinValue = 0, MaxValue = 8000,
				},
				new ValueDisplayElement
				{
					Name = "RPM Value",
					X = 320, Y = 340,
					FontSize = 42, Color = "#00FFFF",
					Font = "DSEG7 Classic", Format = "F0",
					DataSource = "Rpm", MinValue = 0, MaxValue = 8000,
				},
				new TextElement
				{
					Name = "Title",
					X = 320, Y = 390,
					Text = "RPM", FontSize = 20,
					Color = "#666666", Font = "Race Sport",
				},
				new CircleElement
				{
					Name = "Hub Cap",
					X = 320, Y = 240,
					Radius = 8, FillColor = "#444444",
					StrokeColor = "#666666", CircleStrokeWidth = 1,
				},
			]
		};
	}
}