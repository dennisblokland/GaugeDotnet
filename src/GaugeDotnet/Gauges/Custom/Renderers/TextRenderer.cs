using SkiaSharp;

namespace GaugeDotnet.Gauges.Custom.Rendering;

internal static class TextRenderer
{
	internal static void DrawText(SKCanvas canvas, TextElement text)
	{
		SKPaint paint = RenderContext.Paint;
		SKFont font = RenderContext.Font;
		font.Typeface = RenderContext.GetTypeface(text.Font);
		font.Size = text.FontSize;

		if (text.ShowBox)
		{
			float textW = font.MeasureText(text.Text);
			float bx = text.X - textW / 2 - text.BoxPadding;
			float by = text.Y - text.FontSize - text.BoxPadding;
			float bw = textW + text.BoxPadding * 2;
			float bh = text.FontSize * 1.3f + text.BoxPadding * 2;
			paint.Style = SKPaintStyle.Fill;
			paint.Color = SKColor.Parse(text.BoxColor);
			paint.MaskFilter = null;
			if (text.BoxCornerRadius > 0)
				canvas.DrawRoundRect(bx, by, bw, bh, text.BoxCornerRadius, text.BoxCornerRadius, paint);
			else
				canvas.DrawRect(bx, by, bw, bh, paint);
		}

		paint.Style = SKPaintStyle.Fill;
		paint.Color = SKColor.Parse(text.Color);
		paint.MaskFilter = null;
		canvas.DrawText(text.Text, text.X, text.Y, SKTextAlign.Center, font, paint);
	}

	internal static void DrawValueDisplay(SKCanvas canvas, ValueDisplayElement display, float value)
	{
		SKPaint paint = RenderContext.Paint;
		SKFont font = RenderContext.Font;
		string formatted = value.ToString(display.Format) + display.Suffix;
		font.Typeface = RenderContext.GetTypeface(display.Font);
		font.Size = display.FontSize;
		paint.Style = SKPaintStyle.Fill;
		paint.Color = SKColor.Parse(display.Color);
		paint.MaskFilter = null;
		canvas.DrawText(formatted, display.X, display.Y, SKTextAlign.Center, font, paint);
	}

	internal static void DrawLabelValue(SKCanvas canvas, LabelValueElement lv, float value)
	{
		SKPaint paint = RenderContext.Paint;
		SKFont font = RenderContext.Font;
		string valueText = value.ToString(lv.ValueFormat) + lv.ValueSuffix;

		SKTypeface labelTypeface = RenderContext.GetTypeface(lv.LabelFont);
		SKTypeface valueTypeface = RenderContext.GetTypeface(lv.ValueFont);

		using SKFont measureFont = new(labelTypeface, lv.LabelFontSize);
		using SKFont measureValueFont = new(valueTypeface, lv.ValueFontSize);
		float labelW = measureFont.MeasureText(lv.Label);
		float valueW = measureValueFont.MeasureText(valueText);

		float gap = 4f;
		float totalH = lv.LabelFontSize + gap + lv.ValueFontSize;
		float totalW = MathF.Max(labelW, valueW);

		if (lv.ShowBox)
		{
			float bx = lv.X - totalW / 2 - lv.BoxPadding;
			float by = lv.Y - totalH / 2 - lv.BoxPadding;
			float bw = totalW + lv.BoxPadding * 2;
			float bh = totalH + lv.BoxPadding * 2;
			paint.Style = SKPaintStyle.Fill;
			paint.Color = SKColor.Parse(lv.BoxColor);
			paint.MaskFilter = null;
			if (lv.BoxCornerRadius > 0)
				canvas.DrawRoundRect(bx, by, bw, bh, lv.BoxCornerRadius, lv.BoxCornerRadius, paint);
			else
				canvas.DrawRect(bx, by, bw, bh, paint);
		}

		float labelY = lv.Y - totalH / 2 + lv.LabelFontSize;
		float valueY = lv.Y + totalH / 2;

		font.Typeface = labelTypeface;
		font.Size = lv.LabelFontSize;
		paint.Style = SKPaintStyle.Fill;
		paint.Color = SKColor.Parse(lv.LabelColor);
		paint.MaskFilter = null;
		canvas.DrawText(lv.Label, lv.X, labelY, SKTextAlign.Center, font, paint);

		font.Typeface = valueTypeface;
		font.Size = lv.ValueFontSize;
		paint.Color = SKColor.Parse(lv.ValueColor);
		canvas.DrawText(valueText, lv.X, valueY, SKTextAlign.Center, font, paint);
	}
}
