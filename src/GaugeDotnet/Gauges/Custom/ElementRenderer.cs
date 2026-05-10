using System;
using System.Collections.Generic;
using GaugeDotnet.Rendering;
using SkiaSharp;

namespace GaugeDotnet.Gauges.Custom;

public static class ElementRenderer
{
	public static void Render(
		SKCanvas canvas,
		CustomGaugeDefinition definition,
		Dictionary<string, float> values,
		Dictionary<string, float>? targetValues = null)
	{
		canvas.Clear(SKColor.Parse(definition.BackgroundColor));

		foreach (GaugeElement element in definition.Elements)
		{
			bool useTarget = targetValues is not null
				&& (element is ValueDisplayElement or TextElement);
			Dictionary<string, float> source = useTarget ? targetValues! : values;

			float value = 0f;
			if (!string.IsNullOrEmpty(element.DataSource) &&
				source.TryGetValue(element.DataSource, out float v))
			{
				value = v;
			}
			DrawElement(canvas, element, value);
		}
	}

	public static void DrawElement(SKCanvas canvas, GaugeElement element, float value)
	{
		switch (element)
		{
			case ArcElement arc: DrawArc(canvas, arc, value); break;
			case NeedleElement needle: DrawNeedle(canvas, needle, value); break;
			case TextElement text: DrawText(canvas, text); break;
			case ValueDisplayElement val: DrawValueDisplay(canvas, val, value); break;
			case TickRingElement ticks: DrawTickRing(canvas, ticks); break;
			case CircleElement circle: DrawCircle(canvas, circle); break;
		}
	}

	private static void DrawArc(SKCanvas canvas, ArcElement arc, float value)
	{
		SKRect rect = new(
			arc.X - arc.Radius, arc.Y - arc.Radius,
			arc.X + arc.Radius, arc.Y + arc.Radius);

		if (arc.ShowTrack)
		{
			using SKPaint trackPaint = new()
			{
				Style = SKPaintStyle.Stroke,
				StrokeWidth = arc.StrokeWidth,
				StrokeCap = SKStrokeCap.Butt,
				Color = SKColor.Parse(arc.TrackColor),
				IsAntialias = true,
			};
			canvas.DrawArc(rect, arc.StartAngleDeg, arc.SweepAngleDeg, false, trackPaint);
		}

		float fillSweep = arc.SweepAngleDeg;
		if (arc.IsDynamic && !string.IsNullOrEmpty(arc.DataSource))
		{
			float range = arc.MaxValue - arc.MinValue;
			float t = range > 0 ? Math.Clamp((value - arc.MinValue) / range, 0f, 1f) : 0f;
			fillSweep = arc.SweepAngleDeg * t;
		}

		using SKPaint fillPaint = new()
		{
			Style = SKPaintStyle.Stroke,
			StrokeWidth = arc.StrokeWidth,
			StrokeCap = SKStrokeCap.Butt,
			Color = SKColor.Parse(arc.Color),
			IsAntialias = true,
		};
		canvas.DrawArc(rect, arc.StartAngleDeg, fillSweep, false, fillPaint);

		if (fillSweep > 0)
		{
			SKColor glowColor = SKColor.Parse(arc.Color).WithAlpha(60);
			using SKPaint glowPaint = new()
			{
				Style = SKPaintStyle.Stroke,
				StrokeWidth = arc.StrokeWidth + 12,
				StrokeCap = SKStrokeCap.Butt,
				Color = glowColor,
				IsAntialias = true,
				MaskFilter = SKMaskFilter.CreateBlur(SKBlurStyle.Normal, 8),
			};
			canvas.DrawArc(rect, arc.StartAngleDeg, fillSweep, false, glowPaint);
		}
	}

	private static void DrawNeedle(SKCanvas canvas, NeedleElement needle, float value)
	{
		float t = 0f;
		if (!string.IsNullOrEmpty(needle.DataSource))
		{
			float range = needle.MaxValue - needle.MinValue;
			t = range > 0 ? Math.Clamp((value - needle.MinValue) / range, 0f, 1f) : 0f;
		}

		float angleDeg = needle.StartAngleDeg + t * needle.SweepAngleDeg;
		float angleRad = angleDeg * MathF.PI / 180f;

		float tipX = needle.X + MathF.Cos(angleRad) * needle.Length;
		float tipY = needle.Y + MathF.Sin(angleRad) * needle.Length;
		float tailX = needle.X - MathF.Cos(angleRad) * needle.TailLength;
		float tailY = needle.Y - MathF.Sin(angleRad) * needle.TailLength;

		using SKPaint glowPaint = new()
		{
			Style = SKPaintStyle.Stroke,
			StrokeWidth = needle.NeedleWidth + 6,
			StrokeCap = SKStrokeCap.Round,
			Color = SKColor.Parse(needle.Color).WithAlpha(50),
			IsAntialias = true,
			MaskFilter = SKMaskFilter.CreateBlur(SKBlurStyle.Normal, 6),
		};
		canvas.DrawLine(tailX, tailY, tipX, tipY, glowPaint);

		using SKPaint needlePaint = new()
		{
			Style = SKPaintStyle.Stroke,
			StrokeWidth = needle.NeedleWidth,
			StrokeCap = SKStrokeCap.Round,
			Color = SKColor.Parse(needle.Color),
			IsAntialias = true,
		};
		canvas.DrawLine(tailX, tailY, tipX, tipY, needlePaint);

		if (needle.ShowHub)
		{
			using SKPaint hubPaint = new()
			{
				Style = SKPaintStyle.Fill,
				Color = SKColor.Parse(needle.HubColor),
				IsAntialias = true,
			};
			canvas.DrawCircle(needle.X, needle.Y, needle.HubRadius, hubPaint);
		}
	}

	private static void DrawText(SKCanvas canvas, TextElement text)
	{
		SKTypeface typeface = GetTypeface(text.Font);
		using SKFont font = new(typeface, text.FontSize);
		using SKPaint paint = new()
		{
			Color = SKColor.Parse(text.Color),
			IsAntialias = true,
		};
		canvas.DrawText(text.Text, text.X, text.Y, SKTextAlign.Center, font, paint);
	}

	private static void DrawValueDisplay(SKCanvas canvas, ValueDisplayElement display, float value)
	{
		string formatted = value.ToString(display.Format) + display.Suffix;
		SKTypeface typeface = GetTypeface(display.Font);
		using SKFont font = new(typeface, display.FontSize);
		using SKPaint paint = new()
		{
			Color = SKColor.Parse(display.Color),
			IsAntialias = true,
		};
		canvas.DrawText(formatted, display.X, display.Y, SKTextAlign.Center, font, paint);
	}

	private static void DrawTickRing(SKCanvas canvas, TickRingElement ticks)
	{
		int totalMajor = ticks.MajorCount;

		for (int i = 0; i <= totalMajor; i++)
		{
			float t = (float)i / totalMajor;
			float angleDeg = ticks.StartAngleDeg + t * ticks.SweepAngleDeg;
			float angleRad = angleDeg * MathF.PI / 180f;

			float cos = MathF.Cos(angleRad);
			float sin = MathF.Sin(angleRad);

			float outerX = ticks.X + cos * ticks.Radius;
			float outerY = ticks.Y + sin * ticks.Radius;
			float innerX = ticks.X + cos * (ticks.Radius - ticks.MajorLength);
			float innerY = ticks.Y + sin * (ticks.Radius - ticks.MajorLength);

			using SKPaint majorPaint = new()
			{
				Style = SKPaintStyle.Stroke,
				StrokeWidth = ticks.MajorWidth,
				Color = SKColor.Parse(ticks.Color),
				IsAntialias = true,
			};
			canvas.DrawLine(outerX, outerY, innerX, innerY, majorPaint);

			if (ticks.ShowLabels)
			{
				float labelValue = ticks.MinValue + t * (ticks.MaxValue - ticks.MinValue);
				string label = labelValue.ToString("F0");
				float labelDist = ticks.Radius - ticks.MajorLength - ticks.LabelOffset;
				float labelX = ticks.X + cos * labelDist;
				float labelY = ticks.Y + sin * labelDist;

				using SKFont labelFont = new(SKTypeface.Default, ticks.LabelFontSize);
				using SKPaint labelPaint = new()
				{
					Color = SKColor.Parse(ticks.LabelColor),
					IsAntialias = true,
				};
				canvas.DrawText(label, labelX, labelY + ticks.LabelFontSize / 3f, SKTextAlign.Center, labelFont, labelPaint);
			}

			if (i < totalMajor && ticks.MinorPerMajor > 0)
			{
				for (int j = 1; j <= ticks.MinorPerMajor; j++)
				{
					float mt = t + ((float)j / (ticks.MinorPerMajor + 1)) / totalMajor;
					float mAngleRad = (ticks.StartAngleDeg + mt * ticks.SweepAngleDeg) * MathF.PI / 180f;
					float mCos = MathF.Cos(mAngleRad);
					float mSin = MathF.Sin(mAngleRad);

					float mOuterX = ticks.X + mCos * ticks.Radius;
					float mOuterY = ticks.Y + mSin * ticks.Radius;
					float mInnerX = ticks.X + mCos * (ticks.Radius - ticks.MinorLength);
					float mInnerY = ticks.Y + mSin * (ticks.Radius - ticks.MinorLength);

					using SKPaint minorPaint = new()
					{
						Style = SKPaintStyle.Stroke,
						StrokeWidth = ticks.MinorWidth,
						Color = SKColor.Parse(ticks.Color),
						IsAntialias = true,
					};
					canvas.DrawLine(mOuterX, mOuterY, mInnerX, mInnerY, minorPaint);
				}
			}
		}
	}

	private static void DrawCircle(SKCanvas canvas, CircleElement circle)
	{
		using SKPaint fillPaint = new()
		{
			Style = SKPaintStyle.Fill,
			Color = SKColor.Parse(circle.FillColor),
			IsAntialias = true,
		};
		canvas.DrawCircle(circle.X, circle.Y, circle.Radius, fillPaint);

		if (circle.CircleStrokeWidth > 0)
		{
			using SKPaint strokePaint = new()
			{
				Style = SKPaintStyle.Stroke,
				StrokeWidth = circle.CircleStrokeWidth,
				Color = SKColor.Parse(circle.StrokeColor),
				IsAntialias = true,
			};
			canvas.DrawCircle(circle.X, circle.Y, circle.Radius, strokePaint);
		}
	}

	private static SKTypeface GetTypeface(string fontKey)
	{
		try
		{
			return FontHelper.GetFont(fontKey);
		}
		catch
		{
			return SKTypeface.Default;
		}
	}
}
