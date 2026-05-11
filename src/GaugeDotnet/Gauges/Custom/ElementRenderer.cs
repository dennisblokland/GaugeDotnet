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
				SKRect dest = new(0, 0, definition.Width, definition.Height);
				_paint.Style = SKPaintStyle.Fill;
				_paint.MaskFilter = null;
				_paint.Color = SKColors.White;
				canvas.DrawBitmap(bgBitmap, dest, _paint);
			}
		}

		foreach (GaugeElement element in definition.Elements)
		{
			bool useTarget = targetValues is not null
				&& (element is ValueDisplayElement or TextElement or WarningIndicatorElement);
			Dictionary<string, float> source = useTarget ? targetValues! : values;

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
		}
	}

	private static void DrawArc(SKCanvas canvas, ArcElement arc, float value)
	{
		SKRect rect = new(
			arc.X - arc.Radius, arc.Y - arc.Radius,
			arc.X + arc.Radius, arc.Y + arc.Radius);

		if (arc.ShowTrack)
		{
			_paint.Style = SKPaintStyle.Stroke;
			_paint.StrokeWidth = arc.StrokeWidth;
			_paint.StrokeCap = SKStrokeCap.Butt;
			_paint.Color = SKColor.Parse(arc.TrackColor);
			_paint.MaskFilter = null;
			canvas.DrawArc(rect, arc.StartAngleDeg, arc.SweepAngleDeg, false, _paint);
		}

		float fillSweep = arc.SweepAngleDeg;
		if (arc.IsDynamic && !string.IsNullOrEmpty(arc.DataSource))
		{
			float range = arc.MaxValue - arc.MinValue;
			float t = range > 0 ? Math.Clamp((value - arc.MinValue) / range, 0f, 1f) : 0f;
			fillSweep = arc.SweepAngleDeg * t;
		}

		_paint.Style = SKPaintStyle.Stroke;
		_paint.StrokeWidth = arc.StrokeWidth;
		_paint.StrokeCap = SKStrokeCap.Butt;
		_paint.Color = SKColor.Parse(arc.Color);
		_paint.MaskFilter = null;
		canvas.DrawArc(rect, arc.StartAngleDeg, fillSweep, false, _paint);

		if (fillSweep > 0)
		{
			_paint.StrokeWidth = arc.StrokeWidth + 12;
			_paint.Color = SKColor.Parse(arc.Color).WithAlpha(60);
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

		float angleDeg = needle.StartAngleDeg + t * needle.SweepAngleDeg;

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

		_paint.Style = SKPaintStyle.Stroke;
		_paint.MaskFilter = null;

		for (int i = 0; i <= totalMajor; i++)
		{
			float t = (float)i / totalMajor;
			float angleDeg = ticks.StartAngleDeg + t * ticks.SweepAngleDeg;
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
					float mAngleRad = (ticks.StartAngleDeg + mt * ticks.SweepAngleDeg) * MathF.PI / 180f;
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

		if (fillRect.Width > 0 && fillRect.Height > 0)
		{
			canvas.Save();
			if (bar.CornerRadius > 0)
			{
				using SKPathBuilder clipPathBuilder = new();
				clipPathBuilder.AddRoundRect(trackRect, bar.CornerRadius, bar.CornerRadius);
				using SKPath clipPath = clipPathBuilder.Detach();
				canvas.ClipPath(clipPath);
			}

			_paint.Color = SKColor.Parse(bar.FillColor);
			canvas.DrawRect(fillRect, _paint);

			// Glow
			_paint.Color = SKColor.Parse(bar.FillColor).WithAlpha(60);
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
		_paint.Color = new SKColor(255, 255, 255, img.Opacity);
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
		{
			bitmap?.Dispose();
		}
		_imageCache.Clear();

		foreach (SKMaskFilter filter in _blurCache.Values)
		{
			filter.Dispose();
		}
		_blurCache.Clear();
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
