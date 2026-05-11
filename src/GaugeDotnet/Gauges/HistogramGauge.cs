using GaugeDotnet.Rendering;
using GaugeDotnet.Gauges.Components;
using GaugeDotnet.Gauges.Models;
using SkiaSharp;

namespace GaugeDotnet.Gauges
{
	public class HistogramGauge : BaseGauge
	{
		private readonly int _maxDataPoints;
		private readonly int _intervalMs;
		private readonly float[] _dataPoints;
		private int _dataCount;
		private int _dataHead;
		private long _lastUpdateTicks;

		private readonly SegmentDisplay _valueDisplay;
		private readonly SKMaskFilter _blur;
		private readonly SKTypeface _raceFont;

		private readonly SKBitmap _staticBitmap;
		private readonly SKCanvas _staticCanvas;

		private SKColor _cachedActiveColor;
		private SKColor _cachedInactiveColor;

		private const float SHADOW_BLUR = 15f;
		private const float CHART_X = 70f;
		private const float CHART_Y = 150f;
		private const float CHART_WIDTH = 500f;
		private const float CHART_HEIGHT = 200f;

		private readonly SKPaint _borderPaint;
		private readonly SKPaint _barPaint;
		private readonly SKPaint _labelPaint;

		public HistogramGauge(
			HistogramGaugeSettings settings,
			int screenWidth = 640,
			int screenHeight = 480
		) : base(settings)
		{
			_maxDataPoints = settings.MaxDataPoints;
			_intervalMs = settings.IntervalMs;
			_dataPoints = new float[_maxDataPoints];
			Array.Fill(_dataPoints, Value);
			_dataCount = _maxDataPoints;
			_dataHead = 0;
			_lastUpdateTicks = Environment.TickCount64;

			_staticBitmap = new SKBitmap(screenWidth, screenHeight, SKColorType.Rgba8888, SKAlphaType.Premul);
			_staticCanvas = new SKCanvas(_staticBitmap);

			(SKColor activeCol, SKColor inactiveCol) = Colors;
			_valueDisplay = new SegmentDisplay(
				screenWidth,
				screenHeight,
				centerX: 320f,
				centerY: 100f,
				inactiveHex: $"#{inactiveCol.Red:X2}{inactiveCol.Green:X2}{inactiveCol.Blue:X2}",
				activeHex: $"#{activeCol.Red:X2}{activeCol.Green:X2}{activeCol.Blue:X2}",
				shadowBlur: SHADOW_BLUR,
				decimals: settings.Decimals
			);

			_cachedActiveColor = activeCol;
			_cachedInactiveColor = inactiveCol;

			_blur = SKMaskFilter.CreateBlur(SKBlurStyle.Normal, SHADOW_BLUR / 2f);
			_raceFont = FontHelper.GetFont("Race Sport");

			_borderPaint = new SKPaint
			{
				Style = SKPaintStyle.Stroke,
				StrokeWidth = 4f,
				IsAntialias = true,
			};

			_barPaint = new SKPaint
			{
				Style = SKPaintStyle.Fill,
				IsAntialias = true,
			};

			_labelPaint = new SKPaint { IsAntialias = true };
		}

		public override void SetValue(float v)
		{
			base.SetValue(v);

			long now = Environment.TickCount64;
			if (now - _lastUpdateTicks >= _intervalMs)
			{
				_dataPoints[_dataHead] = v;
				_dataHead = (_dataHead + 1) % _maxDataPoints;
				if (_dataCount < _maxDataPoints)
				{
					_dataCount++;
				}
				_lastUpdateTicks = now;
			}
		}

		private void UpdateStaticBackground(int screenWidth, int screenHeight)
		{
			DrawBackground(_staticCanvas, screenWidth, screenHeight);

			(SKColor activeCol, SKColor _) = Colors;

			// Border
			_borderPaint.Color = activeCol;
			_borderPaint.MaskFilter = _blur;
			SKRect borderRect = new(CHART_X, CHART_Y, CHART_X + CHART_WIDTH, CHART_Y + CHART_HEIGHT);
			_staticCanvas.DrawRect(borderRect, _borderPaint);
			_borderPaint.MaskFilter = null;
			_staticCanvas.DrawRect(borderRect, _borderPaint);

			// Unit label
			using SKFont unitFont = new(_raceFont, 30f);
			_labelPaint.Color = activeCol;
			_labelPaint.MaskFilter = _blur;
			_staticCanvas.DrawText(Unit, 320f, CHART_Y + CHART_HEIGHT + 40f, SKTextAlign.Center, unitFont, _labelPaint);
			_labelPaint.MaskFilter = null;
			_staticCanvas.DrawText(Unit, 320f, CHART_Y + CHART_HEIGHT + 40f, SKTextAlign.Center, unitFont, _labelPaint);

			// Title
			using SKFont titleFont = new(FontHelper.GetFont("DSEG14 Classic"), 20f);
			_labelPaint.MaskFilter = _blur;
			_staticCanvas.DrawText(Title, 320f, CHART_Y + CHART_HEIGHT + 80f, SKTextAlign.Center, titleFont, _labelPaint);
			_labelPaint.MaskFilter = null;
			_staticCanvas.DrawText(Title, 320f, CHART_Y + CHART_HEIGHT + 80f, SKTextAlign.Center, titleFont, _labelPaint);

			_valueDisplay.SetColors(Colors.Inactive, activeCol);
			_cachedActiveColor = activeCol;
			_cachedInactiveColor = Colors.Inactive;

			StaticCacheValid = true;
		}

		public override void Draw(SKCanvas canvas)
		{
			(SKColor activeCol, SKColor inactiveCol) = Colors;

			if (!StaticCacheValid)
			{
				UpdateStaticBackground(canvas.DeviceClipBounds.Width, canvas.DeviceClipBounds.Height);
			}

			canvas.DrawBitmap(_staticBitmap, 0, 0);

			// Draw histogram bars
			float barWidth = CHART_WIDTH / _maxDataPoints;
			float range = (float)(MaxValue - MinValue);

			_barPaint.Color = activeCol;
			_barPaint.MaskFilter = _blur;

			for (int i = 0; i < _dataCount; i++)
			{
				// Read from ring buffer in order (oldest first)
				int idx = (_dataHead - _dataCount + i + _maxDataPoints) % _maxDataPoints;
				float pct = range > 0 ? (float)(_dataPoints[idx] - MinValue) / range : 0f;
				pct = Math.Clamp(pct, 0f, 1f);
				float barHeight = pct * CHART_HEIGHT;
				float barX = CHART_X + i * barWidth + 2f;
				float barY = CHART_Y + CHART_HEIGHT - barHeight;
				canvas.DrawRect(barX, barY, barWidth - 4f, barHeight, _barPaint);
			}

			// Draw again without blur for crisp bars
			_barPaint.MaskFilter = null;
			for (int i = 0; i < _dataCount; i++)
			{
				int idx = (_dataHead - _dataCount + i + _maxDataPoints) % _maxDataPoints;
				float pct = range > 0 ? (float)(_dataPoints[idx] - MinValue) / range : 0f;
				pct = Math.Clamp(pct, 0f, 1f);
				float barHeight = pct * CHART_HEIGHT;
				float barX = CHART_X + i * barWidth + 2f;
				float barY = CHART_Y + CHART_HEIGHT - barHeight;
				canvas.DrawRect(barX, barY, barWidth - 4f, barHeight, _barPaint);
			}

			// Draw value display
			_valueDisplay.SetValue(Value);
			if (activeCol != _cachedActiveColor || inactiveCol != _cachedInactiveColor)
			{
				_valueDisplay.SetColors(inactiveCol, activeCol);
				_cachedActiveColor = activeCol;
				_cachedInactiveColor = inactiveCol;
			}
			_valueDisplay.DrawOnCanvas(canvas);
		}

        public override void ResetSavedState()
        {
            // Clear data points
			Array.Fill(_dataPoints, 0f	);
			_dataCount = _maxDataPoints;
			_dataHead = 0;
			_lastUpdateTicks = Environment.TickCount64;
        }
    }
}
