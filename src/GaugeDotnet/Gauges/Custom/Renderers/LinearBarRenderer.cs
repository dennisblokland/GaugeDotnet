using System;
using SkiaSharp;

namespace GaugeDotnet.Gauges.Custom.Rendering;

internal static class LinearBarRenderer
{
	internal static void Draw(SKCanvas canvas, LinearBarElement bar, float value)
	{
		SKPaint paint = RenderContext.Paint;
		float range = bar.MaxValue - bar.MinValue;
		float t = range > 0 ? Math.Clamp((value - bar.MinValue) / range, 0f, 1f) : 0f;

		SKRect trackRect = new(bar.X, bar.Y, bar.X + bar.Width, bar.Y + bar.Height);

		paint.Style = SKPaintStyle.Fill;
		paint.Color = SKColor.Parse(bar.TrackColor);
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

			paint.Color = SKColor.Parse(barFillColor);
			canvas.DrawRect(fillRect, paint);

			paint.Color = SKColor.Parse(barFillColor).WithAlpha(60);
			paint.MaskFilter = RenderContext.GetBlur(6);
			canvas.DrawRect(fillRect, paint);
			canvas.Restore();
		}

		if (bar.BorderWidth > 0)
		{
			paint.Style = SKPaintStyle.Stroke;
			paint.StrokeWidth = bar.BorderWidth;
			paint.Color = SKColor.Parse(bar.BorderColor);
			paint.MaskFilter = null;
			if (bar.CornerRadius > 0)
				canvas.DrawRoundRect(trackRect, bar.CornerRadius, bar.CornerRadius, paint);
			else
				canvas.DrawRect(trackRect, paint);
		}
	}
}
