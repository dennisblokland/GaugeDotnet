using GaugeDotnet.Gauges.Components;
using GaugeDotnet.Gauges.Models;
using SkiaSharp;

namespace GaugeDotnet.Gauges
{
	public class CircularGauge : BaseGauge
	{
		private float _currentValue;
		private readonly bool _smoothing;
		private readonly int _segmentCount;

		private readonly SegmentDisplay _valueDisplay;
		private readonly SKMaskFilter _blur;
		private readonly SKTypeface _raceFont;
		private readonly SKTypeface _dseg14Font;

		private readonly SKBitmap _staticBitmap;
		private readonly SKCanvas _staticCanvas;

		private SKColor _cachedActiveColor;
		private SKColor _cachedInactiveColor;
		// Pre-computed per-segment angles (degrees) — computed once in constructor
		private readonly float[] _segmentStartDeg;
		private readonly float[] _segmentSweepDeg;

		// Pre-computed arc rect from constants
		private readonly SKRect _arcRect;

		// Cached active-segment bitmap — only redrawn when segment count or color changes
		private readonly SKBitmap _activeBitmap;
		private readonly SKCanvas _activeCanvas;
		private int _cachedActiveSegmentCount = -1;
		private SKColor _cachedActiveSegmentColor;

		private const float SHADOW_BLUR = 15f;
		private const float CENTER_X = 320f;
		private const float CENTER_Y = 240f;
		private const float LINE_WIDTH = 40f;
		private const float RADIUS = 240f - LINE_WIDTH / 2f - 10f;
		private const float START_ANGLE_DEG = 120f;
		private const float END_ANGLE_DEG = 420f;

		private static readonly float StartAngleRad = START_ANGLE_DEG * MathF.PI / 180f;
		private static readonly float EndAngleRad = END_ANGLE_DEG * MathF.PI / 180f;
		private static readonly float RangeAngleRad = EndAngleRad - StartAngleRad;
		private const float SEGMENT_MARGIN = 0.01f; // radians

		private readonly SKPaint _arcPaint;
		private readonly SKPaint _glowArcPaint;
		private readonly SKPaint _innerArcPaint;
		private readonly SKPaint _tickPaint;
		private readonly SKPaint _labelPaint;

		public CircularGauge(
			CircularGaugeSettings settings,
			int screenWidth = 640,
			int screenHeight = 480
		) : base(settings)
		{
			_currentValue = Value;
			_smoothing = settings.Smoothing;
			_segmentCount = settings.SegmentCount;

			// Pre-compute per-segment start angle and sweep (degrees) — reused every frame
			_segmentStartDeg = new float[_segmentCount];
			_segmentSweepDeg = new float[_segmentCount];
			for (int i = 0; i < _segmentCount; i++)
			{
				float segStartRad = StartAngleRad + ((float)i / _segmentCount) * RangeAngleRad + SEGMENT_MARGIN / 2f;
				float segEndRad = StartAngleRad + ((float)(i + 1) / _segmentCount) * RangeAngleRad - SEGMENT_MARGIN / 2f;
				_segmentStartDeg[i] = segStartRad * 180f / MathF.PI;
				_segmentSweepDeg[i] = (segEndRad - segStartRad) * 180f / MathF.PI;
			}

			// Arc rect is derived purely from constants — compute once
			_arcRect = new SKRect(CENTER_X - RADIUS, CENTER_Y - RADIUS, CENTER_X + RADIUS, CENTER_Y + RADIUS);

			_staticBitmap = new SKBitmap(screenWidth, screenHeight, SKColorType.Rgba8888, SKAlphaType.Premul);
			_staticCanvas = new SKCanvas(_staticBitmap);

			_activeBitmap = new SKBitmap(screenWidth, screenHeight, SKColorType.Rgba8888, SKAlphaType.Premul);
			_activeCanvas = new SKCanvas(_activeBitmap);

			(SKColor activeCol, SKColor inactiveCol) = Colors;
			_valueDisplay = new SegmentDisplay(
				screenWidth,
				screenHeight,
				centerX: CENTER_X,
				centerY: CENTER_Y,
				inactiveHex: $"#{inactiveCol.Red:X2}{inactiveCol.Green:X2}{inactiveCol.Blue:X2}",
				activeHex: $"#{activeCol.Red:X2}{activeCol.Green:X2}{activeCol.Blue:X2}",
				shadowBlur: SHADOW_BLUR,
				decimals: settings.Decimals
			);

			_cachedActiveColor = activeCol;
			_cachedInactiveColor = inactiveCol;

			_blur = SKMaskFilter.CreateBlur(SKBlurStyle.Normal, SHADOW_BLUR);
			_raceFont = FontHelper.GetFont("Race Sport");
			_dseg14Font = FontHelper.GetFont("DSEG14 Classic");

			_arcPaint = new SKPaint
			{
				Style = SKPaintStyle.Stroke,
				StrokeWidth = LINE_WIDTH,
				IsAntialias = true,
				StrokeCap = SKStrokeCap.Butt,
			};

			_glowArcPaint = new SKPaint
			{
				Style = SKPaintStyle.Stroke,
				StrokeWidth = LINE_WIDTH,
				IsAntialias = true,
				StrokeCap = SKStrokeCap.Butt,
			};

			_innerArcPaint = new SKPaint
			{
				Style = SKPaintStyle.Stroke,
				StrokeWidth = 2f,
				IsAntialias = true,
			};

			_tickPaint = new SKPaint { IsAntialias = true };
			_labelPaint = new SKPaint { IsAntialias = true };
		}

		private void UpdateStaticBackground(int screenWidth, int screenHeight)
		{
			DrawBackground(_staticCanvas, screenWidth, screenHeight);

			(SKColor activeCol, SKColor inactiveCol) = Colors;

			// 1. Draw inactive segmented arc
			_arcPaint.Color = inactiveCol;
			_arcPaint.MaskFilter = null;
			for (int i = 0; i < _segmentCount; i++)
			{
				_staticCanvas.DrawArc(_arcRect, _segmentStartDeg[i], _segmentSweepDeg[i], false, _arcPaint);
			}

			// 2. Draw inner arc with glow
			float innerRadius = RADIUS - LINE_WIDTH / 2f - 5f;
			SKRect innerRect = new(
				CENTER_X - innerRadius, CENTER_Y - innerRadius,
				CENTER_X + innerRadius, CENTER_Y + innerRadius
			);
			_innerArcPaint.Color = activeCol;
			_innerArcPaint.MaskFilter = _blur;
			float startDegFull = START_ANGLE_DEG;
			float sweepDegFull = END_ANGLE_DEG - START_ANGLE_DEG;
			_staticCanvas.DrawArc(innerRect, startDegFull, sweepDegFull, false, _innerArcPaint);
			_innerArcPaint.MaskFilter = null;
			_staticCanvas.DrawArc(innerRect, startDegFull, sweepDegFull, false, _innerArcPaint);

			// 3. Draw tick labels
			_tickPaint.Color = activeCol;
			_tickPaint.MaskFilter = _blur;
			using SKFont tickFont = new(_raceFont, 14f);
			float step = (float)(MaxValue - MinValue) / 10f;
			for (int i = 0; i <= 10; i++)
			{
				float tickValue = MinValue + i * step;
				float angle = StartAngleRad + ((float)i / 10f) * RangeAngleRad;
				string text = $"{Math.Round(tickValue)}";
				float textWidth = tickFont.MeasureText(text);
				float tickX = CENTER_X + MathF.Cos(angle) * (RADIUS - (LINE_WIDTH + textWidth / 4f));
				float tickY = CENTER_Y + MathF.Sin(angle) * (RADIUS - (LINE_WIDTH + 18f / 4f));
				_staticCanvas.DrawText(text, tickX, tickY + 6f, SKTextAlign.Center, tickFont, _tickPaint);
			}
			_tickPaint.MaskFilter = null;
			for (int i = 0; i <= 10; i++)
			{
				float tickValue = MinValue + i * step;
				float angle = StartAngleRad + ((float)i / 10f) * RangeAngleRad;
				string text = $"{Math.Round(tickValue)}";
				float textWidth = tickFont.MeasureText(text);
				float tickX = CENTER_X + MathF.Cos(angle) * (RADIUS - (LINE_WIDTH + textWidth / 4f));
				float tickY = CENTER_Y + MathF.Sin(angle) * (RADIUS - (LINE_WIDTH + 18f / 4f));
				_staticCanvas.DrawText(text, tickX, tickY + 6f, SKTextAlign.Center, tickFont, _tickPaint);
			}

			// 4. Draw unit label
			using SKFont unitFont = new(_dseg14Font, 25f);
			// Inactive placeholder
			string placeholder = new('~', Unit.Length);
			_labelPaint.Color = inactiveCol;
			_labelPaint.MaskFilter = null;
			_staticCanvas.DrawText(placeholder, CENTER_X, CENTER_Y + RADIUS - 50f, SKTextAlign.Center, unitFont, _labelPaint);
			// Active
			_labelPaint.Color = activeCol;
			_labelPaint.MaskFilter = _blur;
			_staticCanvas.DrawText(Unit, CENTER_X, CENTER_Y + RADIUS - 50f, SKTextAlign.Center, unitFont, _labelPaint);
			_labelPaint.MaskFilter = null;
			_staticCanvas.DrawText(Unit, CENTER_X, CENTER_Y + RADIUS - 50f, SKTextAlign.Center, unitFont, _labelPaint);

			// 5. Draw title
			using SKFont titleFont = new(_raceFont, 15f);
			_labelPaint.Color = activeCol;
			_labelPaint.MaskFilter = _blur;
			_staticCanvas.DrawText(Title, CENTER_X, CENTER_Y + RADIUS - 15f, SKTextAlign.Center, titleFont, _labelPaint);
			_labelPaint.MaskFilter = null;
			_staticCanvas.DrawText(Title, CENTER_X, CENTER_Y + RADIUS - 15f, SKTextAlign.Center, titleFont, _labelPaint);

			_valueDisplay.SetColors(inactiveCol, activeCol);
			_cachedActiveColor = activeCol;
			_cachedInactiveColor = inactiveCol;

			// Invalidate active segment cache so it redraws with the new color
			_cachedActiveSegmentCount = -1;

			StaticCacheValid = true;
		}

		public override void Draw(SKCanvas canvas)
		{
			(SKColor activeCol, SKColor inactiveCol) = Colors;

			// Smooth value
			if (_smoothing)
			{
				_currentValue += (Value - _currentValue) * 0.1f;
			}
			else
			{
				_currentValue = Value;
			}

			if (!StaticCacheValid)
			{
				UpdateStaticBackground(canvas.DeviceClipBounds.Width, canvas.DeviceClipBounds.Height);
			}

				canvas.DrawBitmap(_staticBitmap, 0, 0);

			// Draw active segments — only regenerate when count or color changes
			float pct = (float)((_currentValue - MinValue) / (MaxValue - MinValue));
			pct = Math.Clamp(pct, 0f, 1f);
			int activeSegments = (int)MathF.Round(pct * _segmentCount);

			if (activeSegments != _cachedActiveSegmentCount || activeCol != _cachedActiveSegmentColor)
			{
				_activeCanvas.Clear(SKColors.Transparent);

				if (activeSegments > 0)
				{
					_glowArcPaint.Color = activeCol;
					_glowArcPaint.MaskFilter = _blur;
					for (int i = 0; i < activeSegments; i++)
					{
						_activeCanvas.DrawArc(_arcRect, _segmentStartDeg[i], _segmentSweepDeg[i], false, _glowArcPaint);
					}

					_glowArcPaint.MaskFilter = null;
					for (int i = 0; i < activeSegments; i++)
					{
						_activeCanvas.DrawArc(_arcRect, _segmentStartDeg[i], _segmentSweepDeg[i], false, _glowArcPaint);
					}
				}

				_cachedActiveSegmentCount = activeSegments;
				_cachedActiveSegmentColor = activeCol;
			}

			canvas.DrawBitmap(_activeBitmap, 0, 0);

			_valueDisplay.SetValue(Value);
			if (activeCol != _cachedActiveColor || inactiveCol != _cachedInactiveColor)
			{
				_valueDisplay.SetColors(inactiveCol, activeCol);
				_cachedActiveColor = activeCol;
				_cachedInactiveColor = inactiveCol;
			}
			_valueDisplay.DrawOnCanvas(canvas);
		}
	}
}
