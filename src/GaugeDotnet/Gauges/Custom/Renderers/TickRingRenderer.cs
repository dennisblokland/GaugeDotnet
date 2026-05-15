using GaugeDotnet.Rendering;
using SkiaSharp;

namespace GaugeDotnet.Gauges.Custom.Rendering;

internal static class TickRingRenderer
{
	internal static void Draw(SKCanvas canvas, TickRingElement ticks)
	{
		SKPaint paint = RenderContext.Paint;
		SKFont font = RenderContext.Font;
		int totalMajor = ticks.MajorCount;
		float sign = ticks.AntiClockwise ? -1f : 1f;

		paint.Style = SKPaintStyle.Stroke;
		paint.MaskFilter = null;

		for (int i = 0; i <= totalMajor; i++)
		{
			float t = (float)i / totalMajor;
			float angleDeg = ticks.StartAngleDeg + t * ticks.SweepAngleDeg * sign;
			float angleRad = angleDeg * MathF.PI / 180f;

			float cos = MathF.Cos(angleRad);
			float sin = MathF.Sin(angleRad);

			float outerX = ticks.X + cos * ticks.Radius;
			float outerY = ticks.Y + sin * ticks.Radius;
			float innerX, innerY;
			if (ticks.TicksInside)
			{
				innerX = ticks.X + cos * (ticks.Radius + ticks.MajorLength);
				innerY = ticks.Y + sin * (ticks.Radius + ticks.MajorLength);
			}
			else
			{
				innerX = ticks.X + cos * (ticks.Radius - ticks.MajorLength);
				innerY = ticks.Y + sin * (ticks.Radius - ticks.MajorLength);
			}

			if (ticks.ShowTicks)
			{
				paint.StrokeWidth = ticks.MajorWidth;
				paint.StrokeCap = SKStrokeCap.Butt;
				paint.Color = SKColor.Parse(ticks.Color);
				canvas.DrawLine(outerX, outerY, innerX, innerY, paint);
			}

			if (ticks.ShowLabels)
			{
				float labelValue = ticks.MinValue + t * (ticks.MaxValue - ticks.MinValue);
				string label = labelValue.ToString("F0");
				float labelDist = ticks.TicksInside
					? ticks.Radius + ticks.MajorLength + ticks.LabelOffset
					: ticks.Radius - ticks.MajorLength - ticks.LabelOffset;
				float labelX = ticks.X + cos * labelDist;
				float labelY = ticks.Y + sin * labelDist;

				font.Typeface = FontHelper.Default;
				font.Size = ticks.LabelFontSize;
				paint.Style = SKPaintStyle.Fill;
				paint.Color = SKColor.Parse(ticks.LabelColor);

				if (ticks.RadialLabels)
				{
					canvas.Save();
					canvas.Translate(labelX, labelY);
					canvas.RotateDegrees(angleDeg + 90);
					canvas.DrawText(label, 0, ticks.LabelFontSize / 3f, SKTextAlign.Center, font, paint);
					canvas.Restore();
				}
				else
				{
					canvas.DrawText(label, labelX, labelY + ticks.LabelFontSize / 3f, SKTextAlign.Center, font, paint);
				}

				paint.Style = SKPaintStyle.Stroke;
			}

			if (i < totalMajor && ticks.MinorPerMajor > 0 && ticks.ShowTicks)
			{
				paint.StrokeWidth = ticks.MinorWidth;
				paint.Color = SKColor.Parse(ticks.Color);

				for (int j = 1; j <= ticks.MinorPerMajor; j++)
				{
					float mt = t + ((float)j / (ticks.MinorPerMajor + 1)) / totalMajor;
					float mAngleRad = (ticks.StartAngleDeg + mt * ticks.SweepAngleDeg * sign) * MathF.PI / 180f;
					float mCos = MathF.Cos(mAngleRad);
					float mSin = MathF.Sin(mAngleRad);

					float mOuterX = ticks.X + mCos * ticks.Radius;
					float mOuterY = ticks.Y + mSin * ticks.Radius;
					float mInnerX, mInnerY;
					if (ticks.TicksInside)
					{
						mInnerX = ticks.X + mCos * (ticks.Radius + ticks.MinorLength);
						mInnerY = ticks.Y + mSin * (ticks.Radius + ticks.MinorLength);
					}
					else
					{
						mInnerX = ticks.X + mCos * (ticks.Radius - ticks.MinorLength);
						mInnerY = ticks.Y + mSin * (ticks.Radius - ticks.MinorLength);
					}

					canvas.DrawLine(mOuterX, mOuterY, mInnerX, mInnerY, paint);
				}
			}
		}
	}
}
