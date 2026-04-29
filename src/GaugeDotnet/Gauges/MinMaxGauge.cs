using GaugeDotnet.Extensions;
using GaugeDotnet.Gauges.Components;
using GaugeDotnet.Gauges.Models;
using SkiaSharp;

namespace GaugeDotnet.Gauges
{
    /// <summary>
    /// Bar gauge that tracks and displays min/max peak markers over time.
    /// Shows the current value bar plus persistent min and max indicators.
    /// </summary>
    public class MinMaxGauge : BaseGauge
    {
        private float _currentValue;
        private float _minTracked;
        private float _maxTracked;
        private bool _peaksInitialized;
        private readonly bool _smoothing;
        private readonly int _segmentCount;
        private readonly float _segmentWidth;

        private readonly SegmentDisplay _valueDisplay;
        private readonly SKMaskFilter _blur;
        private readonly SKTypeface _raceFont;

        private readonly SKBitmap _staticBitmap;
        private readonly SKCanvas _staticCanvas;

        private SKColor _cachedActiveColor;
        private SKColor _cachedInactiveColor;

        private const float SHADOW_BLUR = 15f;
        private const float BAR_X = 70f;
        private const float BAR_Y = 210f;
        private const float BAR_WIDTH = 500f;
        private const float BAR_HEIGHT = 60f;

        private readonly SKPaint _inactiveSegmentPaint;
        private readonly SKPaint _activeSegmentPaint;
        private readonly SKPaint _borderPaint;
        private readonly SKPaint _labelPaint;
        private readonly SKPaint _markerPaint;
        private readonly SKPaint _peakLabelPaint;

        public MinMaxGauge(
            MinMaxGaugeSettings settings,
            int screenWidth = 640,
            int screenHeight = 480
        ) : base(settings)
        {
            _currentValue = Value;
            _minTracked = Value;
            _maxTracked = Value;
            _peaksInitialized = false;
            _smoothing = settings.Smoothing;
            _segmentCount = settings.SegmentCount;
            _segmentWidth = BAR_WIDTH / _segmentCount;

            _staticBitmap = new SKBitmap(screenWidth, screenHeight, SKColorType.Rgba8888, SKAlphaType.Premul);
            _staticCanvas = new SKCanvas(_staticBitmap);

            (SKColor activeCol, SKColor inactiveCol) = Colors;
            _valueDisplay = new SegmentDisplay(
                screenWidth,
                screenHeight,
                centerX: 320f,
                centerY: 120f,
                inactiveHex: $"#{inactiveCol.Red:X2}{inactiveCol.Green:X2}{inactiveCol.Blue:X2}",
                activeHex: $"#{activeCol.Red:X2}{activeCol.Green:X2}{activeCol.Blue:X2}",
                shadowBlur: SHADOW_BLUR,
                decimals: settings.Decimals
            );

            _cachedActiveColor = activeCol;
            _cachedInactiveColor = inactiveCol;

            _blur = SKMaskFilter.CreateBlur(SKBlurStyle.Normal, SHADOW_BLUR);
            _raceFont = FontHelper.GetFont("Race Sport");

            _inactiveSegmentPaint = new SKPaint { Style = SKPaintStyle.Fill, Color = inactiveCol, IsAntialias = true };
            _activeSegmentPaint = new SKPaint { Style = SKPaintStyle.Fill, Color = activeCol, IsAntialias = true };
            _borderPaint = new SKPaint { Style = SKPaintStyle.Stroke, Color = activeCol, StrokeWidth = 4f, IsAntialias = true };
            _labelPaint = new SKPaint { Color = activeCol, IsAntialias = true };
            _markerPaint = new SKPaint { Style = SKPaintStyle.Fill, IsAntialias = true };
            _peakLabelPaint = new SKPaint { IsAntialias = true };
        }

        public override void SetValue(float v)
        {
            base.SetValue(v);

            if (!_peaksInitialized)
            {
                _minTracked = v;
                _maxTracked = v;
                _peaksInitialized = true;
            }
            else
            {
                if (v < _minTracked) _minTracked = v;
                if (v > _maxTracked) _maxTracked = v;
            }
        }

        private void UpdateStaticBackground(int screenWidth, int screenHeight)
        {
            DrawBackground(_staticCanvas, screenWidth, screenHeight);

            (SKColor activeCol, SKColor inactiveCol) = Colors;

            // Draw inactive segments
            _inactiveSegmentPaint.Color = inactiveCol;
            for (int i = 0; i < _segmentCount; i++)
            {
                float segX = BAR_X + i * _segmentWidth;
                SKRect rect = new(segX + 2f, BAR_Y, segX + _segmentWidth - 2f, BAR_Y + BAR_HEIGHT);
                _staticCanvas.DrawRect(rect, _inactiveSegmentPaint);
            }

            // Draw border
            _borderPaint.Color = activeCol;
            SKRect borderRect = new(BAR_X - 4f, BAR_Y - 4f, BAR_X + BAR_WIDTH + 4f, BAR_Y + BAR_HEIGHT + 4f);
            _staticCanvas.DrawRectWithBlur(borderRect, _borderPaint, _blur);

            // Unit label
            _labelPaint.Color = activeCol;
            using SKFont unitFont = new(_raceFont, 30f);
            float unitY = BAR_Y + BAR_HEIGHT + 80f;
            _staticCanvas.DrawTextWithBlur(Unit, 320f, unitY, SKTextAlign.Center, unitFont, _labelPaint, _blur);

            // Title
            using SKFont titleFont = new(_raceFont, 20f);
            float titleY = unitY + 40f;
            _staticCanvas.DrawTextWithBlur(Title, 320f, titleY, SKTextAlign.Center, titleFont, _labelPaint, _blur);

            _valueDisplay.SetColors(inactiveCol, activeCol);
            _cachedActiveColor = activeCol;
            _cachedInactiveColor = inactiveCol;

            StaticCacheValid = true;
        }

        public override void Draw(SKCanvas canvas)
        {
            (SKColor activeCol, SKColor inactiveCol) = Colors;

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

            // Draw active segments
            _activeSegmentPaint.Color = activeCol;
            float pct = (float)((_currentValue - MinValue) / (MaxValue - MinValue));
            pct = Math.Clamp(pct, 0f, 1f);
            int activeSegs = (int)Math.Round(pct * _segmentCount);

            for (int i = 0; i < activeSegs; i++)
            {
                float segX = BAR_X + i * _segmentWidth;
                SKRect rect = new(segX + 2f, BAR_Y, segX + _segmentWidth - 2f, BAR_Y + BAR_HEIGHT);
                canvas.DrawRectWithBlur(rect, _activeSegmentPaint, _blur);
            }

            // Draw min marker (triangle below bar)
            float range = (float)(MaxValue - MinValue);
            if (range > 0 && _peaksInitialized)
            {
                float minPct = Math.Clamp((float)(_minTracked - MinValue) / range, 0f, 1f);
                float minX = BAR_X + minPct * BAR_WIDTH;
                DrawMarker(canvas, minX, BAR_Y + BAR_HEIGHT + 6f, SKColors.DeepSkyBlue, true);

                // Min label
                using SKFont peakFont = new(_raceFont, 12f);
                _peakLabelPaint.Color = SKColors.DeepSkyBlue;
                canvas.DrawText($"MIN:{_minTracked:F1}", minX, BAR_Y + BAR_HEIGHT + 38f, SKTextAlign.Center, peakFont, _peakLabelPaint);

                // Draw max marker (triangle above bar)
                float maxPct = Math.Clamp((float)(_maxTracked - MinValue) / range, 0f, 1f);
                float maxX = BAR_X + maxPct * BAR_WIDTH;
                DrawMarker(canvas, maxX, BAR_Y - 6f, SKColors.OrangeRed, false);

                using SKFont peakFont2 = new(_raceFont, 12f);
                _peakLabelPaint.Color = SKColors.OrangeRed;
                canvas.DrawText($"MAX:{_maxTracked:F1}", maxX, BAR_Y - 18f, SKTextAlign.Center, peakFont2, _peakLabelPaint);
            }

            // Value display
            _valueDisplay.SetValue(Value);
            if (activeCol != _cachedActiveColor || inactiveCol != _cachedInactiveColor)
            {
                _valueDisplay.SetColors(inactiveCol, activeCol);
                _cachedActiveColor = activeCol;
                _cachedInactiveColor = inactiveCol;
            }
            _valueDisplay.DrawOnCanvas(canvas);
        }

        private void DrawMarker(SKCanvas canvas, float x, float y, SKColor color, bool pointDown)
        {
            _markerPaint.Color = color;
            _markerPaint.MaskFilter = _blur;

            float dir = pointDown ? 1f : -1f;
            using SKPath path = new();
            path.MoveTo(x, y + dir * 12f);
            path.LineTo(x - 6f, y);
            path.LineTo(x + 6f, y);
            path.Close();

            canvas.DrawPath(path, _markerPaint);
            _markerPaint.MaskFilter = null;
            canvas.DrawPath(path, _markerPaint);
        }
    }
}
