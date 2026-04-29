using GaugeDotnet.Gauges.Componets;
using GaugeDotnet.Gauges.Models;
using SkiaSharp;

namespace GaugeDotnet.Gauges
{
    /// <summary>
    /// Solid filled arc gauge — smooth modern look without segments.
    /// A continuous sweep from start to the current value position.
    /// </summary>
    public class SweepGauge : BaseGauge
    {
        private float _currentValue;
        private readonly bool _smoothing;

        private readonly SegmentDisplay _valueDisplay;
        private readonly SKMaskFilter _blur;
        private readonly SKTypeface _raceFont;
        private readonly SKTypeface _dseg14Font;

        private readonly SKBitmap _staticBitmap;
        private readonly SKCanvas _staticCanvas;

        private SKColor _cachedActiveColor;
        private SKColor _cachedInactiveColor;

        private const float SHADOW_BLUR = 15f;
        private const float CENTER_X = 320f;
        private const float CENTER_Y = 255f;
        private const float OUTER_RADIUS = 210f;
        private const float ARC_WIDTH = 35f;
        private const float START_ANGLE_DEG = 135f;
        private const float END_ANGLE_DEG = 405f;
        private const float SWEEP_RANGE_DEG = END_ANGLE_DEG - START_ANGLE_DEG;

        private static readonly float StartAngleRad = START_ANGLE_DEG * MathF.PI / 180f;
        private static readonly float RangeAngleRad = (END_ANGLE_DEG - START_ANGLE_DEG) * MathF.PI / 180f;

        private readonly SKPaint _trackPaint;
        private readonly SKPaint _fillPaint;
        private readonly SKPaint _glowPaint;
        private readonly SKPaint _dotPaint;
        private readonly SKPaint _labelPaint;

        public SweepGauge(
            SweepGaugeSettings settings,
            int screenWidth = 640,
            int screenHeight = 480
        ) : base(settings)
        {
            _currentValue = Value;
            _smoothing = settings.Smoothing;

            _staticBitmap = new SKBitmap(screenWidth, screenHeight, SKColorType.Rgba8888, SKAlphaType.Premul);
            _staticCanvas = new SKCanvas(_staticBitmap);

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

            _trackPaint = new SKPaint
            {
                Style = SKPaintStyle.Stroke,
                StrokeWidth = ARC_WIDTH,
                StrokeCap = SKStrokeCap.Round,
                IsAntialias = true,
            };

            _fillPaint = new SKPaint
            {
                Style = SKPaintStyle.Stroke,
                StrokeWidth = ARC_WIDTH,
                StrokeCap = SKStrokeCap.Round,
                IsAntialias = true,
            };

            _glowPaint = new SKPaint
            {
                Style = SKPaintStyle.Stroke,
                StrokeWidth = ARC_WIDTH + 8f,
                StrokeCap = SKStrokeCap.Round,
                IsAntialias = true,
            };

            _dotPaint = new SKPaint
            {
                Style = SKPaintStyle.Fill,
                IsAntialias = true,
            };

            _labelPaint = new SKPaint { IsAntialias = true };
        }

        private void UpdateStaticBackground(int screenWidth, int screenHeight)
        {
            DrawBackground(_staticCanvas, screenWidth, screenHeight);

            (SKColor activeCol, SKColor inactiveCol) = Colors;
            SKRect arcRect = new(
                CENTER_X - OUTER_RADIUS, CENTER_Y - OUTER_RADIUS,
                CENTER_X + OUTER_RADIUS, CENTER_Y + OUTER_RADIUS
            );

            // Draw track (inactive arc)
            _trackPaint.Color = inactiveCol;
            _staticCanvas.DrawArc(arcRect, START_ANGLE_DEG, SWEEP_RANGE_DEG, false, _trackPaint);

            // Draw tick dots at intervals
            _dotPaint.Color = new SKColor(
                (byte)Math.Min(activeCol.Red + 40, 255),
                (byte)Math.Min(activeCol.Green + 40, 255),
                (byte)Math.Min(activeCol.Blue + 40, 255),
                80
            );
            float tickRadius = OUTER_RADIUS + ARC_WIDTH / 2f + 8f;
            for (int i = 0; i <= 10; i++)
            {
                float angle = StartAngleRad + ((float)i / 10f) * RangeAngleRad;
                float dx = CENTER_X + MathF.Cos(angle) * tickRadius;
                float dy = CENTER_Y + MathF.Sin(angle) * tickRadius;
                _staticCanvas.DrawCircle(dx, dy, 3f, _dotPaint);
            }

            // Tick labels
            using SKFont tickFont = new(_raceFont, 14f);
            _labelPaint.Color = activeCol;
            float labelRadius = OUTER_RADIUS - ARC_WIDTH / 2f - 20f;
            for (int i = 0; i <= 10; i += 2)
            {
                float tickValue = MinValue + ((float)i / 10f) * (MaxValue - MinValue);
                float angle = StartAngleRad + ((float)i / 10f) * RangeAngleRad;
                float lx = CENTER_X + MathF.Cos(angle) * labelRadius;
                float ly = CENTER_Y + MathF.Sin(angle) * labelRadius + 5f;
                _staticCanvas.DrawText($"{Math.Round(tickValue)}", lx, ly, SKTextAlign.Center, tickFont, _labelPaint);
            }

            // Unit label (directly under value display)
            using SKFont unitFont = new(_dseg14Font, 22f);
            string placeholder = new('~', Unit.Length);
            _labelPaint.Color = inactiveCol;
            _staticCanvas.DrawText(placeholder, CENTER_X, CENTER_Y + 80f, SKTextAlign.Center, unitFont, _labelPaint);
            _labelPaint.Color = activeCol;
            _labelPaint.MaskFilter = _blur;
            _staticCanvas.DrawText(Unit, CENTER_X, CENTER_Y + 80f, SKTextAlign.Center, unitFont, _labelPaint);
            _labelPaint.MaskFilter = null;
            _staticCanvas.DrawText(Unit, CENTER_X, CENTER_Y + 80f, SKTextAlign.Center, unitFont, _labelPaint);

            // Title (between the opening of the arc at the bottom)
            using SKFont titleFont = new(_raceFont, 14f);
            float titleY = CENTER_Y + OUTER_RADIUS * MathF.Sin(45f * MathF.PI / 180f) + 10f;
            _labelPaint.MaskFilter = _blur;
            _staticCanvas.DrawText(Title, CENTER_X, titleY, SKTextAlign.Center, titleFont, _labelPaint);
            _labelPaint.MaskFilter = null;
            _staticCanvas.DrawText(Title, CENTER_X, titleY, SKTextAlign.Center, titleFont, _labelPaint);

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

            // Draw active sweep
            float pct = (float)((_currentValue - MinValue) / (MaxValue - MinValue));
            pct = Math.Clamp(pct, 0f, 1f);
            float activeSweep = pct * SWEEP_RANGE_DEG;

            if (activeSweep > 0.5f)
            {
                SKRect arcRect = new(
                    CENTER_X - OUTER_RADIUS, CENTER_Y - OUTER_RADIUS,
                    CENTER_X + OUTER_RADIUS, CENTER_Y + OUTER_RADIUS
                );

                // Glow
                _glowPaint.Color = activeCol.WithAlpha(60);
                _glowPaint.MaskFilter = _blur;
                canvas.DrawArc(arcRect, START_ANGLE_DEG, activeSweep, false, _glowPaint);

                // Fill
                _fillPaint.Color = activeCol;
                _fillPaint.MaskFilter = null;
                canvas.DrawArc(arcRect, START_ANGLE_DEG, activeSweep, false, _fillPaint);

                // Leading dot
                float tipAngle = StartAngleRad + pct * RangeAngleRad;
                float tipX = CENTER_X + MathF.Cos(tipAngle) * OUTER_RADIUS;
                float tipY = CENTER_Y + MathF.Sin(tipAngle) * OUTER_RADIUS;
                _dotPaint.Color = SKColors.White;
                _dotPaint.MaskFilter = _blur;
                canvas.DrawCircle(tipX, tipY, 6f, _dotPaint);
                _dotPaint.MaskFilter = null;
                canvas.DrawCircle(tipX, tipY, 4f, _dotPaint);
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
    }
}
