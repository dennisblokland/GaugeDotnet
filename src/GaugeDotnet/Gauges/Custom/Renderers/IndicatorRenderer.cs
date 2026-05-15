using GaugeDotnet.Rendering;
using SkiaSharp;

namespace GaugeDotnet.Gauges.Custom.Rendering;

internal static class IndicatorRenderer
{
	internal static void Draw(SKCanvas canvas, WarningIndicatorElement warn, float value)
	{
		SKPaint paint = RenderContext.Paint;
		SKFont font = RenderContext.Font;
		bool active = warn.TriggerAbove ? value >= warn.Threshold : value <= warn.Threshold;
		string color = active ? warn.ActiveColor : warn.InactiveColor;

		paint.Style = SKPaintStyle.Fill;
		paint.Color = SKColor.Parse(color);
		paint.MaskFilter = null;
		canvas.DrawCircle(warn.X, warn.Y, warn.Radius, paint);

		if (active)
		{
			paint.Color = SKColor.Parse(warn.ActiveColor).WithAlpha(80);
			paint.MaskFilter = RenderContext.GetBlur(10);
			canvas.DrawCircle(warn.X, warn.Y, warn.Radius + 4, paint);
		}

		if (warn.ShowLabel)
		{
			font.Typeface = FontHelper.Default;
			font.Size = warn.LabelFontSize;
			paint.Color = SKColor.Parse(warn.LabelColor);
			paint.MaskFilter = null;
			canvas.DrawText(warn.Label, warn.X, warn.Y + warn.Radius + warn.LabelFontSize + 4,
				SKTextAlign.Center, font, paint);
		}
	}
}
