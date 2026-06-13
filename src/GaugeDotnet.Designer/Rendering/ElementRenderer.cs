using System;
using System.Collections.Generic;
using GaugeDotnet.Gauges.Custom;
using GaugeDotnet.Rendering;
using SkiaSharp;
using CoreRenderer = GaugeDotnet.Gauges.Custom.ElementRenderer;

namespace GaugeDotnet.Designer.Rendering;

/// <summary>
/// Designer-side wrapper: delegates drawing to core ElementRenderer,
/// adds hit testing + selection highlights for the editor.
/// </summary>
public static class ElementRenderer
{
	public static void DrawAll(
		SKCanvas canvas,
		CustomGaugeDefinition definition,
		Dictionary<string, float> testValues,
		GaugeElement? selected,
		IReadOnlySet<GaugeElement>? multiSelected = null)
	{
		CoreRenderer.Render(canvas, definition, testValues);

		if (multiSelected != null)
		{
			foreach (GaugeElement el in multiSelected)
				DrawSelectionHighlight(canvas, el, secondary: true);
		}

		if (selected != null)
			DrawSelectionHighlight(canvas, selected, secondary: false);
	}

	// --- Hit testing ---

	public static bool HitTest(GaugeElement element, float px, float py)
	{
		switch (element)
		{
			case ArcElement arc:
			{
				float dx = px - arc.X;
				float dy = py - arc.Y;
				float dist = MathF.Sqrt(dx * dx + dy * dy);
				return dist >= arc.Radius - arc.StrokeWidth - 10 &&
					   dist <= arc.Radius + arc.StrokeWidth + 10;
			}
			case ZoneArcElement zone:
			{
				float dx = px - zone.X;
				float dy = py - zone.Y;
				float dist = MathF.Sqrt(dx * dx + dy * dy);
				return dist >= zone.Radius - zone.StrokeWidth - 10 &&
					   dist <= zone.Radius + zone.StrokeWidth + 10;
			}
			case PeakMarkerElement peak:
			{
				float dx = px - peak.X;
				float dy = py - peak.Y;
				float dist = MathF.Sqrt(dx * dx + dy * dy);
				return dist >= peak.Radius - peak.StrokeWidth - 10 &&
					   dist <= peak.Radius + peak.StrokeWidth + 10;
			}
			case TickRingElement ticks:
			{
				float dx = px - ticks.X;
				float dy = py - ticks.Y;
				float dist = MathF.Sqrt(dx * dx + dy * dy);
				return dist >= ticks.Radius - ticks.MajorLength - 30 &&
					   dist <= ticks.Radius + 10;
			}
			default:
			{
				SKRect bounds = GetBounds(element);
				bounds.Inflate(8, 8);
				return bounds.Contains(px, py);
			}
		}
	}

	public static SKRect GetBounds(GaugeElement element)
	{
		return element switch
		{
			ArcElement arc => new SKRect(
				arc.X - arc.Radius - arc.StrokeWidth / 2,
				arc.Y - arc.Radius - arc.StrokeWidth / 2,
				arc.X + arc.Radius + arc.StrokeWidth / 2,
				arc.Y + arc.Radius + arc.StrokeWidth / 2),
			NeedleElement needle => new SKRect(
				needle.X - needle.Length,
				needle.Y - needle.Length,
				needle.X + needle.Length,
				needle.Y + needle.Length),
			TextElement text => GetTextBounds(text),
			ClockElement clock => GetClockBounds(clock),
			ValueDisplayElement val => GetValueBounds(val),
			TickRingElement ticks => new SKRect(
				ticks.X - ticks.Radius,
				ticks.Y - ticks.Radius,
				ticks.X + ticks.Radius,
				ticks.Y + ticks.Radius),
			CircleElement circle => new SKRect(
				circle.X - circle.Radius,
				circle.Y - circle.Radius,
				circle.X + circle.Radius,
				circle.Y + circle.Radius),
			RectangleElement rect => new SKRect(
				rect.X, rect.Y,
				rect.X + rect.Width, rect.Y + rect.Height),
			LineElement line => new SKRect(
				MathF.Min(line.X, line.X2) - line.LineWidth,
				MathF.Min(line.Y, line.Y2) - line.LineWidth,
				MathF.Max(line.X, line.X2) + line.LineWidth,
				MathF.Max(line.Y, line.Y2) + line.LineWidth),
			LinearBarElement bar => new SKRect(
				bar.X, bar.Y,
				bar.X + bar.Width, bar.Y + bar.Height),
			WarningIndicatorElement warn => new SKRect(
				warn.X - warn.Radius,
				warn.Y - warn.Radius,
				warn.X + warn.Radius,
				warn.Y + warn.Radius + (warn.ShowLabel ? warn.LabelFontSize + 8 : 0)),
			ImageElement img => new SKRect(
				img.X, img.Y,
				img.X + img.Width, img.Y + img.Height),
			GifElement gif => new SKRect(
				gif.X, gif.Y,
				gif.X + gif.Width, gif.Y + gif.Height),
			ZoneArcElement zone => new SKRect(
				zone.X - zone.Radius - zone.StrokeWidth / 2,
				zone.Y - zone.Radius - zone.StrokeWidth / 2,
				zone.X + zone.Radius + zone.StrokeWidth / 2,
				zone.Y + zone.Radius + zone.StrokeWidth / 2),
			GraphElement graph => new SKRect(
				graph.X, graph.Y,
				graph.X + graph.Width, graph.Y + graph.Height),
			LabelValueElement lv => new SKRect(
				lv.X - lv.ValueFontSize * 3,
				lv.Y - (lv.LabelFontSize + 4 + lv.ValueFontSize) / 2 - lv.BoxPadding,
				lv.X + lv.ValueFontSize * 3,
				lv.Y + (lv.LabelFontSize + 4 + lv.ValueFontSize) / 2 + lv.BoxPadding),
			PeakMarkerElement peak => new SKRect(
				peak.X - peak.Radius - peak.StrokeWidth / 2,
				peak.Y - peak.Radius - peak.StrokeWidth / 2,
				peak.X + peak.Radius + peak.StrokeWidth / 2,
				peak.Y + peak.Radius + peak.StrokeWidth / 2),
			_ => new SKRect(element.X - 20, element.Y - 20, element.X + 20, element.Y + 20),
		};
	}

	// --- Resize: inverse of GetBounds, maps a target rect onto an element's size props ---

	public static void SetBounds(GaugeElement element, SKRect r)
	{
		float cx = (r.Left + r.Right) / 2f;
		float cy = (r.Top + r.Bottom) / 2f;
		float w = MathF.Max(2f, r.Width);
		float h = MathF.Max(2f, r.Height);
		float radius = MathF.Max(5f, MathF.Min(w, h) / 2f);

		switch (element)
		{
			case ArcElement a:             a.X = cx; a.Y = cy; a.Radius = radius; break;
			case ZoneArcElement z:         z.X = cx; z.Y = cy; z.Radius = radius; break;
			case PeakMarkerElement p:      p.X = cx; p.Y = cy; p.Radius = radius; break;
			case TickRingElement t:        t.X = cx; t.Y = cy; t.Radius = radius; break;
			case CircleElement c:          c.X = cx; c.Y = cy; c.Radius = MathF.Max(2f, MathF.Min(w, h) / 2f); break;
			case WarningIndicatorElement wi: wi.X = cx; wi.Y = cy; wi.Radius = MathF.Max(2f, MathF.Min(w, h) / 2f); break;
			case NeedleElement n:          n.X = cx; n.Y = cy; n.Length = radius; break;
			case RectangleElement rect:    rect.X = r.Left; rect.Y = r.Top; rect.Width = w; rect.Height = h; break;
			case LinearBarElement bar:     bar.X = r.Left; bar.Y = r.Top; bar.Width = w; bar.Height = h; break;
			case ImageElement img:         img.X = r.Left; img.Y = r.Top; img.Width = w; img.Height = h; break;
			case GifElement gif:           gif.X = r.Left; gif.Y = r.Top; gif.Width = w; gif.Height = h; break;
			case GraphElement g:           g.X = r.Left; g.Y = r.Top; g.Width = w; g.Height = h; break;
			case LineElement ln:           ln.X = r.Left; ln.Y = r.Top; ln.X2 = r.Right; ln.Y2 = r.Bottom; break;
			case TextElement tx:           tx.FontSize = MathF.Max(6f, h / 1.3f); break;
			case ValueDisplayElement vd:   vd.FontSize = MathF.Max(6f, h / 1.3f); break;
			case ClockElement ck:          ck.FontSize = MathF.Max(6f, h / 1.3f); break;
			case LabelValueElement lv:     lv.ValueFontSize = MathF.Max(6f, h * 0.6f); lv.LabelFontSize = MathF.Max(6f, h * 0.25f); break;
		}
	}

	// --- Selection highlight ---

	private static void DrawSelectionHighlight(SKCanvas canvas, GaugeElement element, bool secondary = false)
	{
		SKRect bounds = GetBounds(element);
		bounds.Inflate(6, 6);

		SKColor lineColor = secondary
			? new SKColor(255, 200, 0, 160)
			: new SKColor(255, 255, 255, 180);

		using SKPaint dashPaint = new()
		{
			Style = SKPaintStyle.Stroke,
			Color = lineColor,
			StrokeWidth = 1.5f,
			PathEffect = SKPathEffect.CreateDash([6f, 4f], 0),
			IsAntialias = true,
		};
		canvas.DrawRect(bounds, dashPaint);

		if (!secondary)
		{
			float hs = 5f;
			using SKPaint handlePaint = new()
			{
				Style = SKPaintStyle.Fill,
				Color = SKColors.White,
			};
			canvas.DrawRect(bounds.Left - hs / 2, bounds.Top - hs / 2, hs, hs, handlePaint);
			canvas.DrawRect(bounds.Right - hs / 2, bounds.Top - hs / 2, hs, hs, handlePaint);
			canvas.DrawRect(bounds.Left - hs / 2, bounds.Bottom - hs / 2, hs, hs, handlePaint);
			canvas.DrawRect(bounds.Right - hs / 2, bounds.Bottom - hs / 2, hs, hs, handlePaint);
		}
	}

	// --- Helpers ---

	private static SKRect GetTextBounds(TextElement text)
	{
		SKTypeface typeface = GetTypeface(text.Font);
		using SKFont font = new(typeface, text.FontSize);
		float width = font.MeasureText(text.Text);
		float height = text.FontSize;
		return new SKRect(
			text.X - width / 2,
			text.Y - height,
			text.X + width / 2,
			text.Y + height * 0.3f);
	}

	private static SKRect GetClockBounds(ClockElement clock)
	{
		SKTypeface typeface = GetTypeface(clock.Font);
		using SKFont font = new(typeface, clock.FontSize);
		string sample = DateTime.Now.ToString("HH:mm:ss");
		float width = font.MeasureText(sample);
		float height = clock.FontSize;
		return new SKRect(
			clock.X - width / 2,
			clock.Y - height,
			clock.X + width / 2,
			clock.Y + height * 0.3f);
	}

	private static SKRect GetValueBounds(ValueDisplayElement val)
	{
		SKTypeface typeface = GetTypeface(val.Font);
		using SKFont font = new(typeface, val.FontSize);
		string sample = "8888" + val.Suffix;
		float width = font.MeasureText(sample);
		float height = val.FontSize;
		return new SKRect(
			val.X - width / 2,
			val.Y - height,
			val.X + width / 2,
			val.Y + height * 0.3f);
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
