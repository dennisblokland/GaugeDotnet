using SkiaSharp;

namespace GaugeDotnet.Gauges.Custom.Rendering;

internal static class ArcRenderer
{
	private const byte GlowAlpha = 60;
	private const float GlowSigma = 8f;
	private const float GlowStrokeExtra = 12f;

	internal static void Draw(SKCanvas canvas, ArcElement arc, float value)
	{
		SKPaint paint = RenderContext.Paint;
		SKRect rect = new(arc.X - arc.Radius, arc.Y - arc.Radius, arc.X + arc.Radius, arc.Y + arc.Radius);
		float sign = arc.AntiClockwise ? -1f : 1f;

		if (arc.ShowTrack)
		{
			paint.Style = SKPaintStyle.Stroke;
			paint.StrokeWidth = arc.StrokeWidth;
			paint.StrokeCap = SKStrokeCap.Butt;
			paint.Color = ColorCache.Get(arc.TrackColor);
			paint.MaskFilter = null;
			canvas.DrawArc(rect, arc.StartAngleDeg, arc.SweepAngleDeg * sign, false, paint);
		}

		float fillSweep = arc.SweepAngleDeg;
		if (arc.IsDynamic && !string.IsNullOrEmpty(arc.DataSource))
		{
			float range = arc.MaxValue - arc.MinValue;
			float t = range > 0 ? Math.Clamp((value - arc.MinValue) / range, 0f, 1f) : 0f;
			fillSweep = arc.SweepAngleDeg * t;
		}
		fillSweep *= sign;

		string fillColor = arc.Color;
		if (arc.UseConditionalColor && !string.IsNullOrEmpty(arc.DataSource))
		{
			if (value >= arc.DangerThreshold) fillColor = arc.DangerColor;
			else if (value >= arc.WarnThreshold) fillColor = arc.WarnColor;
		}

		SKColor fillSKColor = ColorCache.Get(fillColor);

		paint.Style = SKPaintStyle.Stroke;
		paint.StrokeWidth = arc.StrokeWidth;
		paint.StrokeCap = SKStrokeCap.Butt;
		paint.Color = fillSKColor;
		paint.MaskFilter = null;
		canvas.DrawArc(rect, arc.StartAngleDeg, fillSweep, false, paint);

		if (fillSweep != 0)
		{
			paint.StrokeWidth = arc.StrokeWidth + GlowStrokeExtra;
			paint.Color = fillSKColor.WithAlpha(GlowAlpha);
			paint.MaskFilter = RenderContext.GetBlur(GlowSigma);
			canvas.DrawArc(rect, arc.StartAngleDeg, fillSweep, false, paint);
			paint.MaskFilter = null;
		}
	}

	internal static void DrawZone(SKCanvas canvas, ZoneArcElement zone, float value)
	{
		SKPaint paint = RenderContext.Paint;
		SKRect rect = new(zone.X - zone.Radius, zone.Y - zone.Radius, zone.X + zone.Radius, zone.Y + zone.Radius);
		float sign = zone.AntiClockwise ? -1f : 1f;
		float sweepDeg = zone.SweepAngleDeg * sign;
		float range = zone.MaxValue - zone.MinValue;
		float t2 = range > 0 ? Math.Clamp((zone.Zone2Start - zone.MinValue) / range, 0f, 1f) : 0.5f;
		float t3 = range > 0 ? Math.Clamp((zone.Zone3Start - zone.MinValue) / range, 0f, 1f) : 0.85f;

		paint.Style = SKPaintStyle.Stroke;
		paint.StrokeWidth = zone.StrokeWidth;
		paint.StrokeCap = SKStrokeCap.Butt;
		paint.MaskFilter = null;

		float z1EndT = zone.ShowZone2 ? t2 : (zone.ShowZone3 ? t3 : 1f);
		paint.Color = ColorCache.Get(zone.Zone1Color);
		canvas.DrawArc(rect, zone.StartAngleDeg, sweepDeg * z1EndT, false, paint);

		if (zone.ShowZone2)
		{
			float z2EndT = zone.ShowZone3 ? t3 : 1f;
			float z2Sweep = (z2EndT - t2) * sweepDeg;
			paint.Color = ColorCache.Get(zone.Zone2Color);
			canvas.DrawArc(rect, zone.StartAngleDeg + t2 * sweepDeg, z2Sweep, false, paint);
		}

		if (zone.ShowZone3)
		{
			float z3StartT = zone.ShowZone2 ? t3 : t2;
			float z3Sweep = (1f - z3StartT) * sweepDeg;
			paint.Color = ColorCache.Get(zone.Zone3Color);
			canvas.DrawArc(rect, zone.StartAngleDeg + z3StartT * sweepDeg, z3Sweep, false, paint);
		}

		if (zone.ShowPointer && !string.IsNullOrEmpty(zone.DataSource))
		{
			float t = range > 0 ? Math.Clamp((value - zone.MinValue) / range, 0f, 1f) : 0f;
			float angleDeg = zone.StartAngleDeg + t * sweepDeg;
			float angleRad = angleDeg * MathF.PI / 180f;
			float innerR = zone.Radius - zone.StrokeWidth / 2 - 4;
			float outerR = zone.Radius + zone.StrokeWidth / 2 + 4;
			paint.Style = SKPaintStyle.Stroke;
			paint.StrokeWidth = zone.PointerWidth;
			paint.StrokeCap = SKStrokeCap.Round;
			paint.Color = ColorCache.Get(zone.PointerColor);
			canvas.DrawLine(
				zone.X + MathF.Cos(angleRad) * innerR,
				zone.Y + MathF.Sin(angleRad) * innerR,
				zone.X + MathF.Cos(angleRad) * outerR,
				zone.Y + MathF.Sin(angleRad) * outerR,
				paint);
		}
	}
}
