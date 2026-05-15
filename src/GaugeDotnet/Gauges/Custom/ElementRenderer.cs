using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.IO;
using GaugeDotnet.Rendering;
using SkiaSharp;

namespace GaugeDotnet.Gauges.Custom;

public static class ElementRenderer
{
	private static readonly ConcurrentDictionary<string, SKBitmap?> _imageCache = new();
	private static readonly ConcurrentDictionary<float, SKMaskFilter> _blurCache = new();
	private static readonly ConcurrentDictionary<string, Queue<float>> _graphBuffers = new();
	private static readonly ConcurrentDictionary<string, (float Peak, DateTime Seen)> _peakState = new();
	private static string? _baseDirectory;

	// Reusable paint/font objects — rendering is single-threaded
	private static readonly SKPaint _paint = new() { IsAntialias = true };
	private static readonly SKFont _font = new(FontHelper.Default);

	/// <summary>
	/// Set the base directory for resolving relative image paths.
	/// Call once after loading the definition file.
	/// </summary>
	public static void SetBaseDirectory(string? directory)
	{
		_baseDirectory = directory;
	}

	public static void Render(
		SKCanvas canvas,
		CustomGaugeDefinition definition,
		Dictionary<string, float> values,
		Dictionary<string, float>? targetValues = null,
		string? baseDirectory = null)
	{
		string? effectiveBaseDir = baseDirectory ?? _baseDirectory;
		canvas.Clear(SKColor.Parse(definition.BackgroundColor));

		if (!string.IsNullOrEmpty(definition.BackgroundImage))
		{
			SKBitmap? bgBitmap = LoadImage(definition.BackgroundImage, effectiveBaseDir);
			if (bgBitmap != null)
			{
				_paint.Style = SKPaintStyle.Fill;
				_paint.MaskFilter = null;
				_paint.Color = new SKColor(255, 255, 255, definition.BackgroundImageOpacity);
				DrawBackgroundBitmap(canvas, bgBitmap, definition.Width, definition.Height,
					definition.BackgroundImageMode);
			}
		}

		// Merge calculated channels into local copies so the caller's dicts are not mutated
		Dictionary<string, float> allValues = values;
		Dictionary<string, float>? allTargetValues = targetValues;
		if (definition.CalculatedChannels.Count > 0)
		{
			allValues = new Dictionary<string, float>(values, System.StringComparer.OrdinalIgnoreCase);
			if (targetValues != null)
				allTargetValues = new Dictionary<string, float>(targetValues, System.StringComparer.OrdinalIgnoreCase);
			foreach (CalculatedChannel ch in definition.CalculatedChannels)
			{
				if (string.IsNullOrEmpty(ch.Name) || string.IsNullOrEmpty(ch.Expression)) continue;
				try
				{
					float result = ExpressionEvaluator.Evaluate(ch.Expression, allValues);
					allValues[ch.Name] = result;
					if (allTargetValues != null) allTargetValues[ch.Name] = result;
				}
				catch { }
			}
		}

		foreach (GaugeElement element in definition.Elements)
		{
			// Visibility check
			if (element.UseVisibility && !string.IsNullOrEmpty(element.VisibilitySource))
			{
				allValues.TryGetValue(element.VisibilitySource, out float visVal);
				if (visVal < element.VisibleAbove || visVal > element.VisibleBelow) continue;
			}

			bool useTarget = allTargetValues is not null
				&& (element is ValueDisplayElement or TextElement or WarningIndicatorElement);
			Dictionary<string, float> source = useTarget ? allTargetValues! : allValues;

			float value = 0f;
			if (!string.IsNullOrEmpty(element.DataSource) &&
				source.TryGetValue(element.DataSource, out float v))
			{
				value = v;
			}
			DrawElement(canvas, element, value, effectiveBaseDir);
		}
	}

	public static void DrawElement(SKCanvas canvas, GaugeElement element, float value, string? baseDirectory = null)
	{
		bool useLayer = element.Opacity < 255;
		if (useLayer)
		{
			using SKPaint layerPaint = new() { Color = SKColors.White.WithAlpha(element.Opacity) };
			canvas.SaveLayer(layerPaint);
		}

		switch (element)
		{
			case ArcElement arc: DrawArc(canvas, arc, value); break;
			case NeedleElement needle: DrawNeedle(canvas, needle, value, baseDirectory); break;
			case TextElement text: DrawText(canvas, text); break;
			case ValueDisplayElement val: DrawValueDisplay(canvas, val, value); break;
			case TickRingElement ticks: DrawTickRing(canvas, ticks); break;
			case CircleElement circle: DrawCircle(canvas, circle); break;
			case RectangleElement rect: DrawRectangle(canvas, rect); break;
			case LineElement line: DrawLine(canvas, line); break;
			case LinearBarElement bar: DrawLinearBar(canvas, bar, value); break;
			case WarningIndicatorElement warn: DrawWarningIndicator(canvas, warn, value); break;
			case ImageElement img: DrawImage(canvas, img, baseDirectory); break;
			case ZoneArcElement zone: DrawZoneArc(canvas, zone, value); break;
			case GraphElement graph: DrawGraph(canvas, graph, value); break;
			case LabelValueElement lv: DrawLabelValue(canvas, lv, value); break;
			case PeakMarkerElement peak: DrawPeakMarker(canvas, peak, value); break;
		}

		if (useLayer) canvas.Restore();
	}

	private static void DrawArc(SKCanvas canvas, ArcElement arc, float value)
	{
		SKRect rect = new(
			arc.X - arc.Radius, arc.Y - arc.Radius,
			arc.X + arc.Radius, arc.Y + arc.Radius);

		float sign = arc.AntiClockwise ? -1f : 1f;

		if (arc.ShowTrack)
		{
			_paint.Style = SKPaintStyle.Stroke;
			_paint.StrokeWidth = arc.StrokeWidth;
			_paint.StrokeCap = SKStrokeCap.Butt;
			_paint.Color = SKColor.Parse(arc.TrackColor);
			_paint.MaskFilter = null;
			canvas.DrawArc(rect, arc.StartAngleDeg, arc.SweepAngleDeg * sign, false, _paint);
		}

		float fillSweep = arc.SweepAngleDeg;
		if (arc.IsDynamic && !string.IsNullOrEmpty(arc.DataSource))
		{
			float range = arc.MaxValue - arc.MinValue;
			float t = range > 0 ? Math.Clamp((value - arc.MinValue) / range, 0f, 1f) : 0f;
			fillSweep = arc.SweepAngleDeg * t;
		}
		fillSweep *= sign;

		string fillColor = arc.Color;
		if (arc.UseConditionalColor && !string.IsNullOrEmpty(arc.DataSource))
		{
			if (value >= arc.DangerThreshold) fillColor = arc.DangerColor;
			else if (value >= arc.WarnThreshold) fillColor = arc.WarnColor;
		}

		_paint.Style = SKPaintStyle.Stroke;
		_paint.StrokeWidth = arc.StrokeWidth;
		_paint.StrokeCap = SKStrokeCap.Butt;
		_paint.Color = SKColor.Parse(fillColor);
		_paint.MaskFilter = null;
		canvas.DrawArc(rect, arc.StartAngleDeg, fillSweep, false, _paint);

		if (fillSweep != 0)
		{
			_paint.StrokeWidth = arc.StrokeWidth + 12;
			_paint.Color = SKColor.Parse(fillColor).WithAlpha(60);
			_paint.MaskFilter = GetBlur(8);
			canvas.DrawArc(rect, arc.StartAngleDeg, fillSweep, false, _paint);
		}
	}

	private static void DrawNeedle(SKCanvas canvas, NeedleElement needle, float value, string? baseDirectory = null)
	{
		float t = 0f;
		if (!string.IsNullOrEmpty(needle.DataSource))
		{
			float range = needle.MaxValue - needle.MinValue;
			t = range > 0 ? Math.Clamp((value - needle.MinValue) / range, 0f, 1f) : 0f;
		}

		float sign = needle.AntiClockwise ? -1f : 1f;
		float angleDeg = needle.StartAngleDeg + t * needle.SweepAngleDeg * sign;

		if (!string.IsNullOrEmpty(needle.ImagePath))
		{
			SKBitmap? img = LoadImage(needle.ImagePath, baseDirectory);
			if (img != null)
			{
				canvas.Save();
				canvas.Translate(needle.X, needle.Y);
				canvas.RotateDegrees(angleDeg + 90);
				SKRect dest = new(
					-needle.ImageWidth / 2, -needle.ImageLength,
					needle.ImageWidth / 2, 0);
				_paint.Style = SKPaintStyle.Fill;
				_paint.MaskFilter = null;
				_paint.Color = SKColors.White;
				canvas.DrawBitmap(img, dest, _paint);
				canvas.Restore();

				if (needle.ShowHub)
				{
					_paint.Color = SKColor.Parse(needle.HubColor);
					canvas.DrawCircle(needle.X, needle.Y, needle.HubRadius, _paint);
				}
				return;
			}
		}

		float angleRad = angleDeg * MathF.PI / 180f;

		float tipX = needle.X + MathF.Cos(angleRad) * needle.Length;
		float tipY = needle.Y + MathF.Sin(angleRad) * needle.Length;
		float tailX = needle.X - MathF.Cos(angleRad) * needle.TailLength;
		float tailY = needle.Y - MathF.Sin(angleRad) * needle.TailLength;

		_paint.Style = SKPaintStyle.Stroke;
		_paint.StrokeWidth = needle.NeedleWidth + 6;
		_paint.StrokeCap = SKStrokeCap.Round;
		_paint.Color = SKColor.Parse(needle.Color).WithAlpha(50);
		_paint.MaskFilter = GetBlur(6);
		canvas.DrawLine(tailX, tailY, tipX, tipY, _paint);

		_paint.StrokeWidth = needle.NeedleWidth;
		_paint.Color = SKColor.Parse(needle.Color);
		_paint.MaskFilter = null;
		canvas.DrawLine(tailX, tailY, tipX, tipY, _paint);

		if (needle.ShowHub)
		{
			_paint.Style = SKPaintStyle.Fill;
			_paint.Color = SKColor.Parse(needle.HubColor);
			canvas.DrawCircle(needle.X, needle.Y, needle.HubRadius, _paint);
		}
	}

	private static void DrawText(SKCanvas canvas, TextElement text)
	{
		SKTypeface typeface = GetTypeface(text.Font);
		_font.Typeface = typeface;
		_font.Size = text.FontSize;

		if (text.ShowBox)
		{
			float textW = _font.MeasureText(text.Text);
			float bx = text.X - textW / 2 - text.BoxPadding;
			float by = text.Y - text.FontSize - text.BoxPadding;
			float bw = textW + text.BoxPadding * 2;
			float bh = text.FontSize * 1.3f + text.BoxPadding * 2;
			_paint.Style = SKPaintStyle.Fill;
			_paint.Color = SKColor.Parse(text.BoxColor);
			_paint.MaskFilter = null;
			if (text.BoxCornerRadius > 0)
				canvas.DrawRoundRect(bx, by, bw, bh, text.BoxCornerRadius, text.BoxCornerRadius, _paint);
			else
				canvas.DrawRect(bx, by, bw, bh, _paint);
		}

		_paint.Style = SKPaintStyle.Fill;
		_paint.Color = SKColor.Parse(text.Color);
		_paint.MaskFilter = null;
		canvas.DrawText(text.Text, text.X, text.Y, SKTextAlign.Center, _font, _paint);
	}

	private static void DrawValueDisplay(SKCanvas canvas, ValueDisplayElement display, float value)
	{
		string formatted = value.ToString(display.Format) + display.Suffix;
		SKTypeface typeface = GetTypeface(display.Font);
		_font.Typeface = typeface;
		_font.Size = display.FontSize;
		_paint.Style = SKPaintStyle.Fill;
		_paint.Color = SKColor.Parse(display.Color);
		_paint.MaskFilter = null;
		canvas.DrawText(formatted, display.X, display.Y, SKTextAlign.Center, _font, _paint);
	}

	private static void DrawTickRing(SKCanvas canvas, TickRingElement ticks)
	{
		int totalMajor = ticks.MajorCount;
		float sign = ticks.AntiClockwise ? -1f : 1f;

		_paint.Style = SKPaintStyle.Stroke;
		_paint.MaskFilter = null;

		for (int i = 0; i <= totalMajor; i++)
		{
			float t = (float)i / totalMajor;
			float angleDeg = ticks.StartAngleDeg + t * ticks.SweepAngleDeg * sign;
			float angleRad = angleDeg * MathF.PI / 180f;

			float cos = MathF.Cos(angleRad);
			float sin = MathF.Sin(angleRad);

			float outerX = ticks.X + cos * ticks.Radius;
			float outerY = ticks.Y + sin * ticks.Radius;
			float innerX, innerY;
			if (ticks.TicksInside)
			{
				innerX = ticks.X + cos * (ticks.Radius + ticks.MajorLength);
				innerY = ticks.Y + sin * (ticks.Radius + ticks.MajorLength);
			}
			else
			{
				innerX = ticks.X + cos * (ticks.Radius - ticks.MajorLength);
				innerY = ticks.Y + sin * (ticks.Radius - ticks.MajorLength);
			}

			if (ticks.ShowTicks)
			{
				_paint.StrokeWidth = ticks.MajorWidth;
				_paint.StrokeCap = SKStrokeCap.Butt;
				_paint.Color = SKColor.Parse(ticks.Color);
				canvas.DrawLine(outerX, outerY, innerX, innerY, _paint);
			}

			if (ticks.ShowLabels)
			{
				float labelValue = ticks.MinValue + t * (ticks.MaxValue - ticks.MinValue);
				string label = labelValue.ToString("F0");
				float labelDist = ticks.TicksInside
					? ticks.Radius + ticks.MajorLength + ticks.LabelOffset
					: ticks.Radius - ticks.MajorLength - ticks.LabelOffset;
				float labelX = ticks.X + cos * labelDist;
				float labelY = ticks.Y + sin * labelDist;

				_font.Typeface = FontHelper.Default;
				_font.Size = ticks.LabelFontSize;
				_paint.Style = SKPaintStyle.Fill;
				_paint.Color = SKColor.Parse(ticks.LabelColor);

				if (ticks.RadialLabels)
				{
					canvas.Save();
					canvas.Translate(labelX, labelY);
					canvas.RotateDegrees(angleDeg + 90);
					canvas.DrawText(label, 0, ticks.LabelFontSize / 3f, SKTextAlign.Center, _font, _paint);
					canvas.Restore();
				}
				else
				{
					canvas.DrawText(label, labelX, labelY + ticks.LabelFontSize / 3f, SKTextAlign.Center, _font, _paint);
				}

				_paint.Style = SKPaintStyle.Stroke;
			}

			if (i < totalMajor && ticks.MinorPerMajor > 0 && ticks.ShowTicks)
			{
				_paint.StrokeWidth = ticks.MinorWidth;
				_paint.Color = SKColor.Parse(ticks.Color);

				for (int j = 1; j <= ticks.MinorPerMajor; j++)
				{
					float mt = t + ((float)j / (ticks.MinorPerMajor + 1)) / totalMajor;
					float mAngleRad = (ticks.StartAngleDeg + mt * ticks.SweepAngleDeg * sign) * MathF.PI / 180f;
					float mCos = MathF.Cos(mAngleRad);
					float mSin = MathF.Sin(mAngleRad);

					float mOuterX = ticks.X + mCos * ticks.Radius;
					float mOuterY = ticks.Y + mSin * ticks.Radius;
					float mInnerX, mInnerY;
					if (ticks.TicksInside)
					{
						mInnerX = ticks.X + mCos * (ticks.Radius + ticks.MinorLength);
						mInnerY = ticks.Y + mSin * (ticks.Radius + ticks.MinorLength);
					}
					else
					{
						mInnerX = ticks.X + mCos * (ticks.Radius - ticks.MinorLength);
						mInnerY = ticks.Y + mSin * (ticks.Radius - ticks.MinorLength);
					}

					canvas.DrawLine(mOuterX, mOuterY, mInnerX, mInnerY, _paint);
				}
			}
		}
	}

	private static void DrawCircle(SKCanvas canvas, CircleElement circle)
	{
		_paint.Style = SKPaintStyle.Fill;
		_paint.Color = SKColor.Parse(circle.FillColor);
		_paint.MaskFilter = null;
		canvas.DrawCircle(circle.X, circle.Y, circle.Radius, _paint);

		if (circle.CircleStrokeWidth > 0)
		{
			_paint.Style = SKPaintStyle.Stroke;
			_paint.StrokeWidth = circle.CircleStrokeWidth;
			_paint.Color = SKColor.Parse(circle.StrokeColor);
			canvas.DrawCircle(circle.X, circle.Y, circle.Radius, _paint);
		}
	}

	private static void DrawRectangle(SKCanvas canvas, RectangleElement rect)
	{
		SKRect bounds = new(rect.X, rect.Y, rect.X + rect.Width, rect.Y + rect.Height);

		_paint.Style = SKPaintStyle.Fill;
		_paint.Color = SKColor.Parse(rect.FillColor);
		_paint.MaskFilter = null;

		if (rect.CornerRadius > 0)
		{
			canvas.DrawRoundRect(bounds, rect.CornerRadius, rect.CornerRadius, _paint);
		}
		else
		{
			canvas.DrawRect(bounds, _paint);
		}

		if (rect.RectStrokeWidth > 0)
		{
			_paint.Style = SKPaintStyle.Stroke;
			_paint.StrokeWidth = rect.RectStrokeWidth;
			_paint.Color = SKColor.Parse(rect.StrokeColor);

			if (rect.CornerRadius > 0)
			{
				canvas.DrawRoundRect(bounds, rect.CornerRadius, rect.CornerRadius, _paint);
			}
			else
			{
				canvas.DrawRect(bounds, _paint);
			}
		}
	}

	private static void DrawLine(SKCanvas canvas, LineElement line)
	{
		_paint.Style = SKPaintStyle.Stroke;
		_paint.StrokeWidth = line.LineWidth;
		_paint.StrokeCap = SKStrokeCap.Round;
		_paint.Color = SKColor.Parse(line.Color);
		_paint.MaskFilter = null;
		canvas.DrawLine(line.X, line.Y, line.X2, line.Y2, _paint);
	}

	private static void DrawLinearBar(SKCanvas canvas, LinearBarElement bar, float value)
	{
		float range = bar.MaxValue - bar.MinValue;
		float t = range > 0 ? Math.Clamp((value - bar.MinValue) / range, 0f, 1f) : 0f;

		SKRect trackRect = new(bar.X, bar.Y, bar.X + bar.Width, bar.Y + bar.Height);

		// Track
		_paint.Style = SKPaintStyle.Fill;
		_paint.Color = SKColor.Parse(bar.TrackColor);
		_paint.MaskFilter = null;
		if (bar.CornerRadius > 0)
		{
			canvas.DrawRoundRect(trackRect, bar.CornerRadius, bar.CornerRadius, _paint);
		}
		else
		{
			canvas.DrawRect(trackRect, _paint);
		}

		// Fill
		SKRect fillRect;
		if (bar.IsVertical)
		{
			float fillH = bar.Height * t;
			fillRect = new SKRect(bar.X, bar.Y + bar.Height - fillH, bar.X + bar.Width, bar.Y + bar.Height);
		}
		else
		{
			float fillW = bar.Width * t;
			fillRect = new SKRect(bar.X, bar.Y, bar.X + fillW, bar.Y + bar.Height);
		}

		string barFillColor = bar.FillColor;
		if (bar.UseConditionalColor && !string.IsNullOrEmpty(bar.DataSource))
		{
			if (value >= bar.DangerThreshold) barFillColor = bar.DangerColor;
			else if (value >= bar.WarnThreshold) barFillColor = bar.WarnColor;
		}

		if (fillRect.Width > 0 && fillRect.Height > 0)
		{
			canvas.Save();
			if (bar.CornerRadius > 0)
			{
				using SKPath clipPath = new();
				clipPath.AddRoundRect(trackRect, bar.CornerRadius, bar.CornerRadius);
				canvas.ClipPath(clipPath);
			}

			_paint.Color = SKColor.Parse(barFillColor);
			canvas.DrawRect(fillRect, _paint);

			// Glow
			_paint.Color = SKColor.Parse(barFillColor).WithAlpha(60);
			_paint.MaskFilter = GetBlur(6);
			canvas.DrawRect(fillRect, _paint);
			canvas.Restore();
		}

		// Border
		if (bar.BorderWidth > 0)
		{
			_paint.Style = SKPaintStyle.Stroke;
			_paint.StrokeWidth = bar.BorderWidth;
			_paint.Color = SKColor.Parse(bar.BorderColor);
			_paint.MaskFilter = null;
			if (bar.CornerRadius > 0)
			{
				canvas.DrawRoundRect(trackRect, bar.CornerRadius, bar.CornerRadius, _paint);
			}
			else
			{
				canvas.DrawRect(trackRect, _paint);
			}
		}
	}

	private static void DrawWarningIndicator(SKCanvas canvas, WarningIndicatorElement warn, float value)
	{
		bool active = warn.TriggerAbove
			? value >= warn.Threshold
			: value <= warn.Threshold;

		string color = active ? warn.ActiveColor : warn.InactiveColor;

		_paint.Style = SKPaintStyle.Fill;
		_paint.Color = SKColor.Parse(color);
		_paint.MaskFilter = null;
		canvas.DrawCircle(warn.X, warn.Y, warn.Radius, _paint);

		if (active)
		{
			_paint.Color = SKColor.Parse(warn.ActiveColor).WithAlpha(80);
			_paint.MaskFilter = GetBlur(10);
			canvas.DrawCircle(warn.X, warn.Y, warn.Radius + 4, _paint);
		}

		if (warn.ShowLabel)
		{
			_font.Typeface = FontHelper.Default;
			_font.Size = warn.LabelFontSize;
			_paint.Color = SKColor.Parse(warn.LabelColor);
			_paint.MaskFilter = null;
			canvas.DrawText(warn.Label, warn.X, warn.Y + warn.Radius + warn.LabelFontSize + 4,
				SKTextAlign.Center, _font, _paint);
		}
	}

	private static void DrawPeakMarker(SKCanvas canvas, PeakMarkerElement peak, float value)
	{
		if (string.IsNullOrEmpty(peak.DataSource)) return;

		(float peakVal, DateTime seen) = _peakState.GetOrAdd(peak.Id, _ => (value, DateTime.UtcNow));

		if (peak.DecaySeconds > 0 && (DateTime.UtcNow - seen).TotalSeconds > peak.DecaySeconds)
		{
			peakVal = value;
			seen = DateTime.UtcNow;
		}

		if (value > peakVal)
		{
			peakVal = value;
			seen = DateTime.UtcNow;
		}

		_peakState[peak.Id] = (peakVal, seen);

		float range = peak.MaxValue - peak.MinValue;
		float t = range > 0 ? Math.Clamp((peakVal - peak.MinValue) / range, 0f, 1f) : 0f;
		float sign = peak.AntiClockwise ? -1f : 1f;
		float angleDeg = peak.StartAngleDeg + t * peak.SweepAngleDeg * sign;
		float angleRad = angleDeg * MathF.PI / 180f;
		float innerR = peak.Radius - peak.StrokeWidth / 2;
		float outerR = peak.Radius + peak.StrokeWidth / 2;

		_paint.Style = SKPaintStyle.Stroke;
		_paint.StrokeWidth = peak.MarkerWidth;
		_paint.StrokeCap = SKStrokeCap.Butt;
		_paint.Color = SKColor.Parse(peak.MarkerColor);
		_paint.MaskFilter = null;
		canvas.DrawLine(
			peak.X + MathF.Cos(angleRad) * innerR,
			peak.Y + MathF.Sin(angleRad) * innerR,
			peak.X + MathF.Cos(angleRad) * outerR,
			peak.Y + MathF.Sin(angleRad) * outerR,
			_paint);
	}

	private static void DrawLabelValue(SKCanvas canvas, LabelValueElement lv, float value)
	{
		string valueText = value.ToString(lv.ValueFormat) + lv.ValueSuffix;

		SKTypeface labelTypeface = GetTypeface(lv.LabelFont);
		SKTypeface valueTypeface = GetTypeface(lv.ValueFont);

		using SKFont measureFont = new(labelTypeface, lv.LabelFontSize);
		using SKFont measureValueFont = new(valueTypeface, lv.ValueFontSize);
		float labelW = measureFont.MeasureText(lv.Label);
		float valueW = measureValueFont.MeasureText(valueText);

		float gap = 4f;
		float totalH = lv.LabelFontSize + gap + lv.ValueFontSize;
		float totalW = MathF.Max(labelW, valueW);

		if (lv.ShowBox)
		{
			float bx = lv.X - totalW / 2 - lv.BoxPadding;
			float by = lv.Y - totalH / 2 - lv.BoxPadding;
			float bw = totalW + lv.BoxPadding * 2;
			float bh = totalH + lv.BoxPadding * 2;
			_paint.Style = SKPaintStyle.Fill;
			_paint.Color = SKColor.Parse(lv.BoxColor);
			_paint.MaskFilter = null;
			if (lv.BoxCornerRadius > 0)
				canvas.DrawRoundRect(bx, by, bw, bh, lv.BoxCornerRadius, lv.BoxCornerRadius, _paint);
			else
				canvas.DrawRect(bx, by, bw, bh, _paint);
		}

		float labelY = lv.Y - totalH / 2 + lv.LabelFontSize;
		float valueY = lv.Y + totalH / 2;

		_font.Typeface = labelTypeface;
		_font.Size = lv.LabelFontSize;
		_paint.Style = SKPaintStyle.Fill;
		_paint.Color = SKColor.Parse(lv.LabelColor);
		_paint.MaskFilter = null;
		canvas.DrawText(lv.Label, lv.X, labelY, SKTextAlign.Center, _font, _paint);

		_font.Typeface = valueTypeface;
		_font.Size = lv.ValueFontSize;
		_paint.Color = SKColor.Parse(lv.ValueColor);
		canvas.DrawText(valueText, lv.X, valueY, SKTextAlign.Center, _font, _paint);
	}

	private static void DrawZoneArc(SKCanvas canvas, ZoneArcElement zone, float value)
	{
		SKRect rect = new(
			zone.X - zone.Radius, zone.Y - zone.Radius,
			zone.X + zone.Radius, zone.Y + zone.Radius);

		float sign = zone.AntiClockwise ? -1f : 1f;
		float sweepDeg = zone.SweepAngleDeg * sign;
		float range = zone.MaxValue - zone.MinValue;
		float t2 = range > 0 ? Math.Clamp((zone.Zone2Start - zone.MinValue) / range, 0f, 1f) : 0.5f;
		float t3 = range > 0 ? Math.Clamp((zone.Zone3Start - zone.MinValue) / range, 0f, 1f) : 0.85f;

		_paint.Style = SKPaintStyle.Stroke;
		_paint.StrokeWidth = zone.StrokeWidth;
		_paint.StrokeCap = SKStrokeCap.Butt;
		_paint.MaskFilter = null;

		// Zone 1 — start → zone2 boundary (or further if later zones disabled)
		float z1EndT = zone.ShowZone2 ? t2 : (zone.ShowZone3 ? t3 : 1f);
		_paint.Color = SKColor.Parse(zone.Zone1Color);
		canvas.DrawArc(rect, zone.StartAngleDeg, sweepDeg * z1EndT, false, _paint);

		if (zone.ShowZone2)
		{
			float z2EndT = zone.ShowZone3 ? t3 : 1f;
			float z2Sweep = (z2EndT - t2) * sweepDeg;
			_paint.Color = SKColor.Parse(zone.Zone2Color);
			canvas.DrawArc(rect, zone.StartAngleDeg + t2 * sweepDeg, z2Sweep, false, _paint);
		}

		if (zone.ShowZone3)
		{
			float z3StartT = zone.ShowZone2 ? t3 : t2;
			float z3Sweep = (1f - z3StartT) * sweepDeg;
			_paint.Color = SKColor.Parse(zone.Zone3Color);
			canvas.DrawArc(rect, zone.StartAngleDeg + z3StartT * sweepDeg, z3Sweep, false, _paint);
		}

		if (zone.ShowPointer && !string.IsNullOrEmpty(zone.DataSource))
		{
			float t = range > 0 ? Math.Clamp((value - zone.MinValue) / range, 0f, 1f) : 0f;
			float angleDeg = zone.StartAngleDeg + t * sweepDeg;
			float angleRad = angleDeg * MathF.PI / 180f;
			float innerR = zone.Radius - zone.StrokeWidth / 2 - 4;
			float outerR = zone.Radius + zone.StrokeWidth / 2 + 4;
			_paint.Style = SKPaintStyle.Stroke;
			_paint.StrokeWidth = zone.PointerWidth;
			_paint.StrokeCap = SKStrokeCap.Round;
			_paint.Color = SKColor.Parse(zone.PointerColor);
			canvas.DrawLine(
				zone.X + MathF.Cos(angleRad) * innerR,
				zone.Y + MathF.Sin(angleRad) * innerR,
				zone.X + MathF.Cos(angleRad) * outerR,
				zone.Y + MathF.Sin(angleRad) * outerR,
				_paint);
		}
	}

	private static void DrawGraph(SKCanvas canvas, GraphElement graph, float value)
	{
		SKRect bounds = new(graph.X, graph.Y, graph.X + graph.Width, graph.Y + graph.Height);

		_paint.Style = SKPaintStyle.Fill;
		_paint.Color = SKColor.Parse(graph.BackColor);
		_paint.MaskFilter = null;
		canvas.DrawRect(bounds, _paint);

		Queue<float> buffer = _graphBuffers.GetOrAdd(graph.Id, _ => new Queue<float>());
		buffer.Enqueue(value);
		while (buffer.Count > Math.Max(graph.HistoryDepth, 2)) buffer.Dequeue();

		if (buffer.Count < 2) return;

		float range = graph.MaxValue - graph.MinValue;
		if (range <= 0) return;

		float[] samples = buffer.ToArray();
		int count = samples.Length;
		float stepX = graph.Width / (graph.HistoryDepth - 1);

		canvas.Save();
		canvas.ClipRect(bounds);

		using SKPath linePath = new();
		for (int i = 0; i < count; i++)
		{
			float x = graph.X + (graph.HistoryDepth - count + i) * stepX;
			float t = Math.Clamp((samples[i] - graph.MinValue) / range, 0f, 1f);
			float y = graph.Y + graph.Height - t * graph.Height;
			if (i == 0) linePath.MoveTo(x, y);
			else linePath.LineTo(x, y);
		}

		if (graph.ShowFill)
		{
			using SKPath fillPath = new(linePath);
			float lastX = graph.X + (graph.HistoryDepth - 1) * stepX;
			float firstX = graph.X + (graph.HistoryDepth - count) * stepX;
			fillPath.LineTo(lastX, graph.Y + graph.Height);
			fillPath.LineTo(firstX, graph.Y + graph.Height);
			fillPath.Close();
			_paint.Style = SKPaintStyle.Fill;
			_paint.Color = SKColor.Parse(graph.FillColor).WithAlpha(graph.FillOpacity);
			canvas.DrawPath(fillPath, _paint);
		}

		_paint.Style = SKPaintStyle.Stroke;
		_paint.StrokeWidth = graph.LineWidth;
		_paint.StrokeCap = SKStrokeCap.Round;
		_paint.StrokeJoin = SKStrokeJoin.Round;
		_paint.Color = SKColor.Parse(graph.LineColor);
		_paint.MaskFilter = null;
		canvas.DrawPath(linePath, _paint);

		canvas.Restore();
	}

	private static void DrawBackgroundBitmap(SKCanvas canvas, SKBitmap bitmap, int canvasW, int canvasH,
		BackgroundImageMode mode)
	{
		float imgW = bitmap.Width;
		float imgH = bitmap.Height;

		switch (mode)
		{
			case BackgroundImageMode.Stretch:
			{
				SKRect dest = new(0, 0, canvasW, canvasH);
				canvas.DrawBitmap(bitmap, dest, _paint);
				break;
			}
			case BackgroundImageMode.Fill:
			{
				float scale = Math.Max(canvasW / imgW, canvasH / imgH);
				float drawW = imgW * scale;
				float drawH = imgH * scale;
				float ox = (canvasW - drawW) / 2f;
				float oy = (canvasH - drawH) / 2f;
				canvas.Save();
				canvas.ClipRect(new SKRect(0, 0, canvasW, canvasH));
				canvas.DrawBitmap(bitmap, new SKRect(ox, oy, ox + drawW, oy + drawH), _paint);
				canvas.Restore();
				break;
			}
			case BackgroundImageMode.Fit:
			{
				float scale = Math.Min(canvasW / imgW, canvasH / imgH);
				float drawW = imgW * scale;
				float drawH = imgH * scale;
				float ox = (canvasW - drawW) / 2f;
				float oy = (canvasH - drawH) / 2f;
				canvas.DrawBitmap(bitmap, new SKRect(ox, oy, ox + drawW, oy + drawH), _paint);
				break;
			}
			case BackgroundImageMode.Center:
			{
				float ox = (canvasW - imgW) / 2f;
				float oy = (canvasH - imgH) / 2f;
				canvas.Save();
				canvas.ClipRect(new SKRect(0, 0, canvasW, canvasH));
				canvas.DrawBitmap(bitmap, new SKRect(ox, oy, ox + imgW, oy + imgH), _paint);
				canvas.Restore();
				break;
			}
			case BackgroundImageMode.Tile:
			{
				canvas.Save();
				canvas.ClipRect(new SKRect(0, 0, canvasW, canvasH));
				for (float ty = 0; ty < canvasH; ty += imgH)
					for (float tx = 0; tx < canvasW; tx += imgW)
						canvas.DrawBitmap(bitmap, new SKRect(tx, ty, tx + imgW, ty + imgH), _paint);
				canvas.Restore();
				break;
			}
		}
	}

	private static void DrawImage(SKCanvas canvas, ImageElement img, string? baseDirectory = null)
	{
		if (string.IsNullOrEmpty(img.ImagePath)) return;

		SKBitmap? bitmap = LoadImage(img.ImagePath, baseDirectory);
		if (bitmap == null) return;

		canvas.Save();

		if (img.Rotation != 0)
		{
			canvas.Translate(img.X + img.Width / 2, img.Y + img.Height / 2);
			canvas.RotateDegrees(img.Rotation);
			canvas.Translate(-(img.X + img.Width / 2), -(img.Y + img.Height / 2));
		}

		SKRect dest = new(img.X, img.Y, img.X + img.Width, img.Y + img.Height);
		_paint.Style = SKPaintStyle.Fill;
		_paint.Color = SKColors.White;
		_paint.MaskFilter = null;
		canvas.DrawBitmap(bitmap, dest, _paint);
		canvas.Restore();
	}

	internal static SKBitmap? LoadImage(string path, string? baseDirectory = null)
	{
		string resolved = ResolveImagePath(path, baseDirectory);
		return _imageCache.GetOrAdd(resolved, p =>
		{
			if (!File.Exists(p)) return null;
			try { return SKBitmap.Decode(p); }
			catch { return null; }
		});
	}

	private static string ResolveImagePath(string path, string? baseDirectory)
	{
		if (Path.IsPathRooted(path)) return path;
		string? dir = baseDirectory ?? _baseDirectory;
		if (dir != null) return Path.Combine(dir, path);
		return Path.Combine(AppContext.BaseDirectory, path);
	}

	/// <summary>
	/// Save a cached image to disk as PNG. Returns false if the image is not in cache.
	/// </summary>
	public static bool SaveImageFromCache(string imagePath, string destPath)
	{
		SKBitmap? bitmap = LoadImage(imagePath, _baseDirectory);
		if (bitmap == null) return false;

		using SKData data = bitmap.Encode(SKEncodedImageFormat.Png, 100);
		using FileStream fs = File.Create(destPath);
		data.SaveTo(fs);
		return true;
	}

	public static void ClearImageCache()
	{
		foreach (SKBitmap? bitmap in _imageCache.Values)
			bitmap?.Dispose();
		_imageCache.Clear();

		foreach (SKMaskFilter filter in _blurCache.Values)
			filter.Dispose();
		_blurCache.Clear();

		_graphBuffers.Clear();
		_peakState.Clear();
	}

	private static SKMaskFilter GetBlur(float sigma)
	{
		return _blurCache.GetOrAdd(sigma, s => SKMaskFilter.CreateBlur(SKBlurStyle.Normal, s));
	}

	private static SKTypeface GetTypeface(string fontKey)
	{
		try
		{
			return FontHelper.GetFont(fontKey);
		}
		catch
		{
			return FontHelper.Default;
		}
	}
}
