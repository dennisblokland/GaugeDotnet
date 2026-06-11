using SkiaSharp;

namespace GaugeDotnet.Gauges.Custom.Rendering;

internal static class ShapeRenderer
{
	internal static void DrawCircle(SKCanvas canvas, CircleElement circle)
	{
		SKPaint paint = RenderContext.Paint;
		paint.Style = SKPaintStyle.Fill;
		paint.Color = ColorCache.Get(circle.FillColor);
		paint.MaskFilter = null;
		canvas.DrawCircle(circle.X, circle.Y, circle.Radius, paint);

		if (circle.CircleStrokeWidth > 0)
		{
			paint.Style = SKPaintStyle.Stroke;
			paint.StrokeWidth = circle.CircleStrokeWidth;
			paint.Color = ColorCache.Get(circle.StrokeColor);
			canvas.DrawCircle(circle.X, circle.Y, circle.Radius, paint);
		}
	}

	internal static void DrawRectangle(SKCanvas canvas, RectangleElement rect)
	{
		SKPaint paint = RenderContext.Paint;
		SKRect bounds = new(rect.X, rect.Y, rect.X + rect.Width, rect.Y + rect.Height);

		paint.Style = SKPaintStyle.Fill;
		paint.Color = ColorCache.Get(rect.FillColor);
		paint.MaskFilter = null;

		if (rect.CornerRadius > 0)
			canvas.DrawRoundRect(bounds, rect.CornerRadius, rect.CornerRadius, paint);
		else
			canvas.DrawRect(bounds, paint);

		if (rect.RectStrokeWidth > 0)
		{
			paint.Style = SKPaintStyle.Stroke;
			paint.StrokeWidth = rect.RectStrokeWidth;
			paint.Color = ColorCache.Get(rect.StrokeColor);

			if (rect.CornerRadius > 0)
				canvas.DrawRoundRect(bounds, rect.CornerRadius, rect.CornerRadius, paint);
			else
				canvas.DrawRect(bounds, paint);
		}
	}

	internal static void DrawLine(SKCanvas canvas, LineElement line)
	{
		SKPaint paint = RenderContext.Paint;
		paint.Style = SKPaintStyle.Stroke;
		paint.StrokeWidth = line.LineWidth;
		paint.StrokeCap = SKStrokeCap.Round;
		paint.Color = ColorCache.Get(line.Color);
		paint.MaskFilter = null;
		canvas.DrawLine(line.X, line.Y, line.X2, line.Y2, paint);
	}
}
