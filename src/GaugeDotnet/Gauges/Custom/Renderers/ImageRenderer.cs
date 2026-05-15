using System;
using SkiaSharp;

namespace GaugeDotnet.Gauges.Custom.Rendering;

internal static class ImageRenderer
{
	internal static void Draw(SKCanvas canvas, ImageElement img, string? baseDirectory = null)
	{
		if (string.IsNullOrEmpty(img.ImagePath)) return;

		SKBitmap? bitmap = ImageCache.Load(img.ImagePath, baseDirectory);
		if (bitmap == null) return;

		SKPaint paint = RenderContext.Paint;
		canvas.Save();

		if (img.Rotation != 0)
		{
			canvas.Translate(img.X + img.Width / 2, img.Y + img.Height / 2);
			canvas.RotateDegrees(img.Rotation);
			canvas.Translate(-(img.X + img.Width / 2), -(img.Y + img.Height / 2));
		}

		SKRect dest = new(img.X, img.Y, img.X + img.Width, img.Y + img.Height);
		paint.Style = SKPaintStyle.Fill;
		paint.Color = SKColors.White;
		paint.MaskFilter = null;
		canvas.DrawBitmap(bitmap, dest, paint);
		canvas.Restore();
	}

	internal static void DrawBackground(SKCanvas canvas, SKBitmap bitmap, int canvasW, int canvasH,
		BackgroundImageMode mode)
	{
		SKPaint paint = RenderContext.Paint;
		float imgW = bitmap.Width;
		float imgH = bitmap.Height;

		switch (mode)
		{
			case BackgroundImageMode.Stretch:
			{
				canvas.DrawBitmap(bitmap, new SKRect(0, 0, canvasW, canvasH), paint);
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
				canvas.DrawBitmap(bitmap, new SKRect(ox, oy, ox + drawW, oy + drawH), paint);
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
				canvas.DrawBitmap(bitmap, new SKRect(ox, oy, ox + drawW, oy + drawH), paint);
				break;
			}
			case BackgroundImageMode.Center:
			{
				float ox = (canvasW - imgW) / 2f;
				float oy = (canvasH - imgH) / 2f;
				canvas.Save();
				canvas.ClipRect(new SKRect(0, 0, canvasW, canvasH));
				canvas.DrawBitmap(bitmap, new SKRect(ox, oy, ox + imgW, oy + imgH), paint);
				canvas.Restore();
				break;
			}
			case BackgroundImageMode.Tile:
			{
				canvas.Save();
				canvas.ClipRect(new SKRect(0, 0, canvasW, canvasH));
				for (float ty = 0; ty < canvasH; ty += imgH)
					for (float tx = 0; tx < canvasW; tx += imgW)
						canvas.DrawBitmap(bitmap, new SKRect(tx, ty, tx + imgW, ty + imgH), paint);
				canvas.Restore();
				break;
			}
		}
	}
}
