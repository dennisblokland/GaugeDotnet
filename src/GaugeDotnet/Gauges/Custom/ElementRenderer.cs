using System;
using System.Collections.Generic;
using GaugeDotnet.Gauges.Custom.Rendering;
using SkiaSharp;

namespace GaugeDotnet.Gauges.Custom;

public static class ElementRenderer
{
	public static void SetBaseDirectory(string? directory)
	{
		ImageCache.BaseDirectory = directory;
	}

	public static void Render(
		SKCanvas canvas,
		CustomGaugeDefinition definition,
		Dictionary<string, float> values,
		Dictionary<string, float>? targetValues = null,
		string? baseDirectory = null)
	{
		string? effectiveBaseDir = baseDirectory ?? ImageCache.BaseDirectory;
		canvas.Clear(SKColor.Parse(definition.BackgroundColor));

		if (!string.IsNullOrEmpty(definition.BackgroundImage))
		{
			SKBitmap? bgBitmap = ImageCache.Load(definition.BackgroundImage, effectiveBaseDir);
			if (bgBitmap != null)
			{
				RenderContext.Paint.Style = SKPaintStyle.Fill;
				RenderContext.Paint.MaskFilter = null;
				RenderContext.Paint.Color = new SKColor(255, 255, 255, definition.BackgroundImageOpacity);
				ImageRenderer.DrawBackground(canvas, bgBitmap, definition.Width, definition.Height,
					definition.BackgroundImageMode);
			}
		}

		// Merge calculated channels into local copies so the caller's dicts are not mutated
		Dictionary<string, float> allValues = values;
		Dictionary<string, float>? allTargetValues = targetValues;
		if (definition.CalculatedChannels.Count > 0)
		{
			allValues = new Dictionary<string, float>(values, StringComparer.OrdinalIgnoreCase);
			if (targetValues != null)
				allTargetValues = new Dictionary<string, float>(targetValues, StringComparer.OrdinalIgnoreCase);
			foreach (CalculatedChannel ch in definition.CalculatedChannels)
			{
				if (string.IsNullOrEmpty(ch.Name) || string.IsNullOrEmpty(ch.Expression)) continue;
				try
				{
					float result = ExpressionEvaluator.Evaluate(ch.Expression, allValues);
					allValues[ch.Name] = result;
					if (allTargetValues != null) allTargetValues[ch.Name] = result;
				}
				catch { }
			}
		}

		foreach (GaugeElement element in definition.Elements)
		{
			if (element.UseVisibility && !string.IsNullOrEmpty(element.VisibilitySource))
			{
				allValues.TryGetValue(element.VisibilitySource, out float visVal);
				if (visVal < element.VisibleAbove || visVal > element.VisibleBelow) continue;
			}

			bool useTarget = allTargetValues is not null
				&& (element is ValueDisplayElement or TextElement or WarningIndicatorElement);
			Dictionary<string, float> source = useTarget ? allTargetValues! : allValues;

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
		bool useLayer = element.Opacity < 255;
		if (useLayer)
		{
			using SKPaint layerPaint = new() { Color = SKColors.White.WithAlpha(element.Opacity) };
			canvas.SaveLayer(layerPaint);
		}

		switch (element)
		{
			case ArcElement arc:             ArcRenderer.Draw(canvas, arc, value); break;
			case NeedleElement needle:       NeedleRenderer.Draw(canvas, needle, value, baseDirectory); break;
			case TextElement text:           TextRenderer.DrawText(canvas, text); break;
			case ValueDisplayElement val:    TextRenderer.DrawValueDisplay(canvas, val, value); break;
			case TickRingElement ticks:      TickRingRenderer.Draw(canvas, ticks); break;
			case CircleElement circle:       ShapeRenderer.DrawCircle(canvas, circle); break;
			case RectangleElement rect:      ShapeRenderer.DrawRectangle(canvas, rect); break;
			case LineElement line:           ShapeRenderer.DrawLine(canvas, line); break;
			case LinearBarElement bar:       LinearBarRenderer.Draw(canvas, bar, value); break;
			case WarningIndicatorElement w:  IndicatorRenderer.Draw(canvas, w, value); break;
			case ImageElement img:           ImageRenderer.Draw(canvas, img, baseDirectory); break;
			case ZoneArcElement zone:        ArcRenderer.DrawZone(canvas, zone, value); break;
			case GraphElement graph:         GraphRenderer.Draw(canvas, graph, value); break;
			case LabelValueElement lv:       TextRenderer.DrawLabelValue(canvas, lv, value); break;
			case PeakMarkerElement peak:     PeakMarkerRenderer.Draw(canvas, peak, value); break;
		}

		if (useLayer) canvas.Restore();
	}

	internal static SKBitmap? LoadImage(string path, string? baseDirectory = null) =>
		ImageCache.Load(path, baseDirectory);

	public static bool SaveImageFromCache(string imagePath, string destPath) =>
		ImageCache.SaveToDisk(imagePath, destPath);

	public static void ClearImageCache()
	{
		ImageCache.Clear();
		RenderContext.ClearBlurCache();
		GraphRenderer.ClearBuffers();
		PeakMarkerRenderer.ClearState();
	}
}
