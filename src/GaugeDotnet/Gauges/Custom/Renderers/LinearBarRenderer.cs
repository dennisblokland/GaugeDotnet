using SkiaSharp;

namespace GaugeDotnet.Gauges.Custom.Rendering;

internal static class LinearBarRenderer
{
	private const byte GlowAlpha = 60;
	private const float GlowSigma = 6f;

	internal static void Draw(SKCanvas canvas, LinearBarElement bar, float value)
	{
		SKPaint paint = RenderContext.Paint;
		float range = bar.MaxValue - bar.MinValue;
		float t = range > 0 ? Math.Clamp((value - bar.MinValue) / range, 0f, 1f) : 0f;

		SKRect trackRect = new(bar.X, bar.Y, bar.X + bar.Width, bar.Y + bar.Height);

		string barFillColor = bar.FillColor;
		if (bar.UseConditionalColor && !string.IsNullOrEmpty(bar.DataSource))
		{
			if (value >= bar.DangerThreshold) barFillColor = bar.DangerColor;
			else if (value >= bar.WarnThreshold) barFillColor = bar.WarnColor;
		}

		if (bar.SegmentCount > 1)
		{
			DrawSegments(canvas, bar, t, ColorCache.Get(barFillColor));
		}
		else
		{
			DrawContinuous(canvas, bar, t, trackRect, barFillColor);
		}

		if (bar.BorderWidth > 0)
		{
			paint.Style = SKPaintStyle.Stroke;
			paint.StrokeWidth = bar.BorderWidth;
			paint.Color = ColorCache.Get(bar.BorderColor);
			paint.MaskFilter = null;
			if (bar.CornerRadius > 0)
				canvas.DrawRoundRect(trackRect, bar.CornerRadius, bar.CornerRadius, paint);
			else
				canvas.DrawRect(trackRect, paint);
		}
	}

	/// <summary>
	/// <see cref="LinearBarElement.SegmentCount"/> segments from the bar's start (left, or bottom when
	/// vertical) separated by <see cref="LinearBarElement.SegmentGap"/> pixels; a segment lights once
	/// the value reaches its end.
	/// </summary>
	private static void DrawSegments(SKCanvas canvas, LinearBarElement bar, float t, SKColor fillSKColor)
	{
		SKPaint paint = RenderContext.Paint;
		int n = bar.SegmentCount;
		int lit = Segments.Lit(n, t);
		float length = bar.IsVertical ? bar.Height : bar.Width;
		float gap = Math.Min(Math.Max(bar.SegmentGap, 0f), 0.8f * length / (n - 1));
		float seg = (length - gap * (n - 1)) / n;
		float step = seg + gap;
		SKColor trackColor = ColorCache.Get(bar.TrackColor);

		SKRect Segment(int first, int count)
		{
			float a = first * step;
			float b = a + count * step - gap;
			return bar.IsVertical
				? new SKRect(bar.X, bar.Y + bar.Height - b, bar.X + bar.Width, bar.Y + bar.Height - a)
				: new SKRect(bar.X + a, bar.Y, bar.X + b, bar.Y + bar.Height);
		}

		paint.Style = SKPaintStyle.Fill;
		paint.MaskFilter = null;
		for (int i = 0; i < n; i++)
		{
			paint.Color = i < lit ? fillSKColor : trackColor;
			SKRect r = Segment(i, 1);
			if (bar.CornerRadius > 0)
				canvas.DrawRoundRect(r, bar.CornerRadius, bar.CornerRadius, paint);
			else
				canvas.DrawRect(r, paint);
		}

		if (lit > 0)
		{
			paint.Color = fillSKColor.WithAlpha(GlowAlpha);
			paint.MaskFilter = RenderContext.GetBlur(GlowSigma);
			canvas.DrawRect(Segment(0, lit), paint);
			paint.MaskFilter = null;
		}
	}

	private static void DrawContinuous(SKCanvas canvas, LinearBarElement bar, float t, SKRect trackRect, string barFillColor)
	{
		SKPaint paint = RenderContext.Paint;
		paint.Style = SKPaintStyle.Fill;
		paint.Color = ColorCache.Get(bar.TrackColor);
		paint.MaskFilter = null;
		if (bar.CornerRadius > 0)
			canvas.DrawRoundRect(trackRect, bar.CornerRadius, bar.CornerRadius, paint);
		else
			canvas.DrawRect(trackRect, paint);

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
			SKColor fillSKColor = ColorCache.Get(barFillColor);

			canvas.Save();
			if (bar.CornerRadius > 0)
			{
				using SKPath clipPath = new();
				clipPath.AddRoundRect(trackRect, bar.CornerRadius, bar.CornerRadius);
				canvas.ClipPath(clipPath);
			}

			paint.Color = fillSKColor;
			canvas.DrawRect(fillRect, paint);

			paint.Color = fillSKColor.WithAlpha(GlowAlpha);
			paint.MaskFilter = RenderContext.GetBlur(GlowSigma);
			canvas.DrawRect(fillRect, paint);
			paint.MaskFilter = null;
			canvas.Restore();
		}
	}
}

internal static class Segments
{
	/// <summary>Segments lit at <paramref name="t"/> (0–1): a segment lights once the value reaches its end.</summary>
	internal static int Lit(int count, float t) => Math.Min((int)MathF.Floor(t * count + 1e-4f), count);
}
