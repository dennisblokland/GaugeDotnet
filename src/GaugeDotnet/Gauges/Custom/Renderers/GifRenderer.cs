using SkiaSharp;

namespace GaugeDotnet.Gauges.Custom.Rendering;

internal static class GifRenderer
{
	internal static void Draw(SKCanvas canvas, GifElement gif, float value, string? baseDirectory = null)
	{
		if (string.IsNullOrEmpty(gif.ImagePath)) return;

		SKBitmap[]? frames = GifCache.Load(gif.ImagePath, baseDirectory);
		if (frames == null || frames.Length == 0) return;

		int index = SelectFrame(value, gif.MinValue, gif.MaxValue, frames.Length);
		SKBitmap bitmap = frames[index];

		SKPaint paint = RenderContext.Paint;
		canvas.Save();

		if (gif.Rotation != 0)
		{
			canvas.Translate(gif.X + gif.Width / 2, gif.Y + gif.Height / 2);
			canvas.RotateDegrees(gif.Rotation);
			canvas.Translate(-(gif.X + gif.Width / 2), -(gif.Y + gif.Height / 2));
		}

		SKRect dest = new(gif.X, gif.Y, gif.X + gif.Width, gif.Y + gif.Height);
		paint.Style = SKPaintStyle.Fill;
		paint.Color = SKColors.White;
		paint.MaskFilter = null;
		canvas.DrawBitmap(bitmap, dest, paint);
		canvas.Restore();
	}

	private static int SelectFrame(float value, float min, float max, int frameCount)
	{
		if (frameCount <= 1) return 0;
		float t = (max > min) ? (value - min) / (max - min) : 0f;
		t = Math.Clamp(t, 0f, 1f);
		int idx = (int)MathF.Round(t * (frameCount - 1));
		return Math.Clamp(idx, 0, frameCount - 1);
	}
}
