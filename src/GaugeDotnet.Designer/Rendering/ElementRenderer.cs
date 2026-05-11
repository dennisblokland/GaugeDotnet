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
		GaugeElement? selected)
	{
		CoreRenderer.Render(canvas, definition, testValues);

		if (selected != null)
		{
			DrawSelectionHighlight(canvas, selected);
		}
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
			_ => new SKRect(element.X - 20, element.Y - 20, element.X + 20, element.Y + 20),
		};
	}

	// --- Selection highlight ---

	private static void DrawSelectionHighlight(SKCanvas canvas, GaugeElement element)
	{
		SKRect bounds = GetBounds(element);
		bounds.Inflate(6, 6);

		using SKPaint dashPaint = new()
		{
			Style = SKPaintStyle.Stroke,
			Color = new SKColor(255, 255, 255, 180),
			StrokeWidth = 1.5f,
			PathEffect = SKPathEffect.CreateDash([6f, 4f], 0),
			IsAntialias = true,
		};
		canvas.DrawRect(bounds, dashPaint);

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
