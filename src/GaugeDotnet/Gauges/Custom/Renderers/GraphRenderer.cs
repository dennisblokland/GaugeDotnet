using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using SkiaSharp;

namespace GaugeDotnet.Gauges.Custom.Rendering;

internal static class GraphRenderer
{
	private static readonly ConcurrentDictionary<string, Queue<float>> _graphBuffers = new();

	internal static void Draw(SKCanvas canvas, GraphElement graph, float value)
	{
		SKPaint paint = RenderContext.Paint;
		SKRect bounds = new(graph.X, graph.Y, graph.X + graph.Width, graph.Y + graph.Height);

		paint.Style = SKPaintStyle.Fill;
		paint.Color = SKColor.Parse(graph.BackColor);
		paint.MaskFilter = null;
		canvas.DrawRect(bounds, paint);

		Queue<float> buffer = _graphBuffers.GetOrAdd(graph.Id, _ => new Queue<float>());
		buffer.Enqueue(value);
		while (buffer.Count > Math.Max(graph.HistoryDepth, 2)) buffer.Dequeue();

		if (buffer.Count < 2) return;

		float range = graph.MaxValue - graph.MinValue;
		if (range <= 0) return;

		float[] samples = buffer.ToArray();
		int count = samples.Length;
		float stepX = graph.Width / (graph.HistoryDepth - 1);

		canvas.Save();
		canvas.ClipRect(bounds);

		using SKPath linePath = new();
		for (int i = 0; i < count; i++)
		{
			float x = graph.X + (graph.HistoryDepth - count + i) * stepX;
			float t = Math.Clamp((samples[i] - graph.MinValue) / range, 0f, 1f);
			float y = graph.Y + graph.Height - t * graph.Height;
			if (i == 0) linePath.MoveTo(x, y);
			else linePath.LineTo(x, y);
		}

		if (graph.ShowFill)
		{
			using SKPath fillPath = new(linePath);
			float lastX = graph.X + (graph.HistoryDepth - 1) * stepX;
			float firstX = graph.X + (graph.HistoryDepth - count) * stepX;
			fillPath.LineTo(lastX, graph.Y + graph.Height);
			fillPath.LineTo(firstX, graph.Y + graph.Height);
			fillPath.Close();
			paint.Style = SKPaintStyle.Fill;
			paint.Color = SKColor.Parse(graph.FillColor).WithAlpha(graph.FillOpacity);
			canvas.DrawPath(fillPath, paint);
		}

		paint.Style = SKPaintStyle.Stroke;
		paint.StrokeWidth = graph.LineWidth;
		paint.StrokeCap = SKStrokeCap.Round;
		paint.StrokeJoin = SKStrokeJoin.Round;
		paint.Color = SKColor.Parse(graph.LineColor);
		paint.MaskFilter = null;
		canvas.DrawPath(linePath, paint);

		canvas.Restore();
	}

	internal static void ClearBuffers() => _graphBuffers.Clear();
}
