using System;
using SkiaSharp;

namespace GaugeDotnet.Gauges.Custom.Rendering;

internal static class NeedleRenderer
{
	internal static void Draw(SKCanvas canvas, NeedleElement needle, float value, string? baseDirectory = null)
	{
		SKPaint paint = RenderContext.Paint;
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
			SKBitmap? img = ImageCache.Load(needle.ImagePath, baseDirectory);
			if (img != null)
			{
				canvas.Save();
				canvas.Translate(needle.X, needle.Y);
				canvas.RotateDegrees(angleDeg + 90);
				SKRect dest = new(-needle.ImageWidth / 2, -needle.ImageLength, needle.ImageWidth / 2, 0);
				paint.Style = SKPaintStyle.Fill;
				paint.MaskFilter = null;
				paint.Color = SKColors.White;
				canvas.DrawBitmap(img, dest, paint);
				canvas.Restore();

				if (needle.ShowHub)
				{
					paint.Color = SKColor.Parse(needle.HubColor);
					canvas.DrawCircle(needle.X, needle.Y, needle.HubRadius, paint);
				}
				return;
			}
		}

		float angleRad = angleDeg * MathF.PI / 180f;
		float tipX = needle.X + MathF.Cos(angleRad) * needle.Length;
		float tipY = needle.Y + MathF.Sin(angleRad) * needle.Length;
		float tailX = needle.X - MathF.Cos(angleRad) * needle.TailLength;
		float tailY = needle.Y - MathF.Sin(angleRad) * needle.TailLength;

		paint.Style = SKPaintStyle.Stroke;
		paint.StrokeWidth = needle.NeedleWidth + 6;
		paint.StrokeCap = SKStrokeCap.Round;
		paint.Color = SKColor.Parse(needle.Color).WithAlpha(50);
		paint.MaskFilter = RenderContext.GetBlur(6);
		canvas.DrawLine(tailX, tailY, tipX, tipY, paint);

		paint.StrokeWidth = needle.NeedleWidth;
		paint.Color = SKColor.Parse(needle.Color);
		paint.MaskFilter = null;
		canvas.DrawLine(tailX, tailY, tipX, tipY, paint);

		if (needle.ShowHub)
		{
			paint.Style = SKPaintStyle.Fill;
			paint.Color = SKColor.Parse(needle.HubColor);
			canvas.DrawCircle(needle.X, needle.Y, needle.HubRadius, paint);
		}
	}
}
