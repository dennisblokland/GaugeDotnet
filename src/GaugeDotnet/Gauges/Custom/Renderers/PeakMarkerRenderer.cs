using System.Collections.Concurrent;
using SkiaSharp;

namespace GaugeDotnet.Gauges.Custom.Rendering;

internal static class PeakMarkerRenderer
{
	private static readonly ConcurrentDictionary<string, (float Peak, DateTime Seen)> _peakState = new();

	internal static void Draw(SKCanvas canvas, PeakMarkerElement peak, float value)
	{
		if (string.IsNullOrEmpty(peak.DataSource)) return;

		DateTime now = DateTime.UtcNow;
		(float peakVal, DateTime seen) = _peakState.GetOrAdd(peak.Id, _ => (value, now));

		if (peak.DecaySeconds > 0 && (now - seen).TotalSeconds > peak.DecaySeconds)
		{
			peakVal = value;
			seen = now;
		}

		if (value > peakVal)
		{
			peakVal = value;
			seen = DateTime.UtcNow;
		}

		_peakState[peak.Id] = (peakVal, seen);

		float range = peak.MaxValue - peak.MinValue;
		float t = range > 0 ? Math.Clamp((peakVal - peak.MinValue) / range, 0f, 1f) : 0f;
		float sign = peak.AntiClockwise ? -1f : 1f;
		float angleDeg = peak.StartAngleDeg + t * peak.SweepAngleDeg * sign;
		float angleRad = angleDeg * MathF.PI / 180f;
		float innerR = peak.Radius - peak.StrokeWidth / 2;
		float outerR = peak.Radius + peak.StrokeWidth / 2;

		SKPaint paint = RenderContext.Paint;
		paint.Style = SKPaintStyle.Stroke;
		paint.StrokeWidth = peak.MarkerWidth;
		paint.StrokeCap = SKStrokeCap.Butt;
		paint.Color = ColorCache.Get(peak.MarkerColor);
		paint.MaskFilter = null;
		canvas.DrawLine(
			peak.X + MathF.Cos(angleRad) * innerR,
			peak.Y + MathF.Sin(angleRad) * innerR,
			peak.X + MathF.Cos(angleRad) * outerR,
			peak.Y + MathF.Sin(angleRad) * outerR,
			paint);
	}

	internal static void ClearState() => _peakState.Clear();
}
