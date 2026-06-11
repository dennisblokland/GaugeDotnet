using GaugeDotnet.Rendering;
using SkiaSharp;

namespace GaugeDotnet.Gauges.Custom.Rendering;

internal static class IndicatorRenderer
{
	private const byte GlowAlpha = 80;
	private const float GlowSigma = 10f;
	private const float GlowRadiusExtra = 4f;

	internal static void Draw(SKCanvas canvas, WarningIndicatorElement warn, float value)
	{
		SKPaint paint = RenderContext.Paint;
		SKFont font = RenderContext.Font;
		bool active = warn.TriggerAbove ? value >= warn.Threshold : value <= warn.Threshold;
		SKColor color = ColorCache.Get(active ? warn.ActiveColor : warn.InactiveColor);

		paint.Style = SKPaintStyle.Fill;
		paint.Color = color;
		paint.MaskFilter = null;
		canvas.DrawCircle(warn.X, warn.Y, warn.Radius, paint);

		if (active)
		{
			paint.Color = ColorCache.Get(warn.ActiveColor).WithAlpha(GlowAlpha);
			paint.MaskFilter = RenderContext.GetBlur(GlowSigma);
			canvas.DrawCircle(warn.X, warn.Y, warn.Radius + GlowRadiusExtra, paint);
			paint.MaskFilter = null;
		}

		if (warn.ShowLabel)
		{
			font.Typeface = FontHelper.Default;
			font.Size = warn.LabelFontSize;
			paint.Color = ColorCache.Get(warn.LabelColor);
			paint.MaskFilter = null;
			canvas.DrawText(warn.Label, warn.X, warn.Y + warn.Radius + warn.LabelFontSize + 4,
				SKTextAlign.Center, font, paint);
		}
	}
}
