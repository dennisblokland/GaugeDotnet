using GaugeDotnet.Extentions;
using GaugeDotnet.Gauges.Componets;
using GaugeDotnet.Gauges.Models;
using SkiaSharp;

namespace GaugeDotnet.Gauges
{
    /// <summary>
    /// Classic analog dial gauge with a rotating needle, tick marks, and arc background.
    /// </summary>
    public class NeedleGauge : BaseGauge
    {
        private decimal _currentValue;
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
        private const float CENTER_Y = 260f;
        private const float RADIUS = 210f;
        private const float START_ANGLE_DEG = 135f;
        private const float END_ANGLE_DEG = 405f;
        private const float NEEDLE_LENGTH = 180f;
        private const float NEEDLE_TAIL = 25f;
        private const float NEEDLE_HALF_WIDTH = 5f;

        private static readonly float StartAngleRad = START_ANGLE_DEG * MathF.PI / 180f;
        private static readonly float EndAngleRad = END_ANGLE_DEG * MathF.PI / 180f;
        private static readonly float RangeAngleRad = EndAngleRad - StartAngleRad;

        private readonly SKPaint _arcPaint;
        private readonly SKPaint _tickPaint;
        private readonly SKPaint _needleFillPaint;
        private readonly SKPaint _needleGlowPaint;
        private readonly SKPaint _hubPaint;
        private readonly SKPaint _labelPaint;

        public NeedleGauge(
            NeedleGaugeSettings settings,
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
                centerY: CENTER_Y + 80f,
                inactiveHex: $"#{inactiveCol.Red:X2}{inactiveCol.Green:X2}{inactiveCol.Blue:X2}",
                activeHex: $"#{activeCol.Red:X2}{activeCol.Green:X2}{activeCol.Blue:X2}",
                shadowBlur: SHADOW_BLUR,
                decimals: settings.Decimals,
                fontSize: 40f
            );

            _cachedActiveColor = activeCol;
            _cachedInactiveColor = inactiveCol;

            _blur = SKMaskFilter.CreateBlur(SKBlurStyle.Normal, SHADOW_BLUR);
            _raceFont = FontHelper.GetFont("Race Sport");
            _dseg14Font = FontHelper.GetFont("DSEG14 Classic");

            _arcPaint = new SKPaint
            {
                Style = SKPaintStyle.Stroke,
                StrokeWidth = 6f,
                IsAntialias = true,
            };

            _tickPaint = new SKPaint { IsAntialias = true };
            _labelPaint = new SKPaint { IsAntialias = true };

            _needleFillPaint = new SKPaint
            {
                Style = SKPaintStyle.Fill,
                IsAntialias = true,
            };

            _needleGlowPaint = new SKPaint
            {
                Style = SKPaintStyle.Fill,
                IsAntialias = true,
            };

            _hubPaint = new SKPaint
            {
                Style = SKPaintStyle.Fill,
                IsAntialias = true,
            };
        }

        private void UpdateStaticBackground(int screenWidth, int screenHeight)
        {
            DrawBackground(_staticCanvas, screenWidth, screenHeight);

            (SKColor activeCol, SKColor inactiveCol) = Colors;
            SKRect arcRect = new(
                CENTER_X - RADIUS, CENTER_Y - RADIUS,
                CENTER_X + RADIUS, CENTER_Y + RADIUS
            );

            // Draw outer arc
            _arcPaint.Color = inactiveCol;
            _arcPaint.MaskFilter = null;
            float sweepDeg = END_ANGLE_DEG - START_ANGLE_DEG;
            _staticCanvas.DrawArc(arcRect, START_ANGLE_DEG, sweepDeg, false, _arcPaint);

            // Draw glowing inner arc
            _arcPaint.Color = activeCol;
            _arcPaint.StrokeWidth = 2f;
            _arcPaint.MaskFilter = _blur;
            float innerR = RADIUS - 15f;
            SKRect innerRect = new(CENTER_X - innerR, CENTER_Y - innerR, CENTER_X + innerR, CENTER_Y + innerR);
            _staticCanvas.DrawArc(innerRect, START_ANGLE_DEG, sweepDeg, false, _arcPaint);
            _arcPaint.MaskFilter = null;
            _staticCanvas.DrawArc(innerRect, START_ANGLE_DEG, sweepDeg, false, _arcPaint);
            _arcPaint.StrokeWidth = 6f;

            // Draw major and minor ticks
            using SKFont tickFont = new(_raceFont, 16f);
            _tickPaint.Color = activeCol;
            for (int i = 0; i <= 10; i++)
            {
                float angle = StartAngleRad + ((float)i / 10f) * RangeAngleRad;
                float cos = MathF.Cos(angle);
                float sin = MathF.Sin(angle);

                // Major tick
                float outerX = CENTER_X + cos * RADIUS;
                float outerY = CENTER_Y + sin * RADIUS;
                float tickInnerX = CENTER_X + cos * (RADIUS - 20f);
                float tickInnerY = CENTER_Y + sin * (RADIUS - 20f);

                _tickPaint.StrokeWidth = 2f;
                _tickPaint.MaskFilter = _blur;
                _staticCanvas.DrawLine(outerX, outerY, tickInnerX, tickInnerY, _tickPaint);
                _tickPaint.MaskFilter = null;
                _staticCanvas.DrawLine(outerX, outerY, tickInnerX, tickInnerY, _tickPaint);

                // Tick label
                decimal tickValue = MinValue + ((decimal)i / 10m) * (MaxValue - MinValue);
                string text = $"{Math.Round(tickValue)}";
                float labelR = RADIUS + 18f;
                float labelX = CENTER_X + cos * labelR;
                float labelY = CENTER_Y + sin * labelR + 6f;
                _tickPaint.MaskFilter = _blur;
                _staticCanvas.DrawText(text, labelX, labelY, SKTextAlign.Center, tickFont, _tickPaint);
                _tickPaint.MaskFilter = null;
                _staticCanvas.DrawText(text, labelX, labelY, SKTextAlign.Center, tickFont, _tickPaint);

                // Minor ticks between majors (except after last)
                if (i < 10)
                {
                    for (int j = 1; j < 5; j++)
                    {
                        float minorAngle = StartAngleRad + ((i * 5f + j) / 50f) * RangeAngleRad;
                        float mCos = MathF.Cos(minorAngle);
                        float mSin = MathF.Sin(minorAngle);
                        float mOuterX = CENTER_X + mCos * RADIUS;
                        float mOuterY = CENTER_Y + mSin * RADIUS;
                        float mInnerX = CENTER_X + mCos * (RADIUS - 10f);
                        float mInnerY = CENTER_Y + mSin * (RADIUS - 10f);

                        _tickPaint.StrokeWidth = 1f;
                        _staticCanvas.DrawLine(mOuterX, mOuterY, mInnerX, mInnerY, _tickPaint);
                    }
                }
            }

            // Draw unit label (below segment display) using segment font
            using SKFont unitFont = new(_dseg14Font, 22f);
            string unitPlaceholder = new('~', Unit.Length);
            _labelPaint.Color = inactiveCol;
            _staticCanvas.DrawText(unitPlaceholder, CENTER_X, CENTER_Y + 130f, SKTextAlign.Center, unitFont, _labelPaint);
            _labelPaint.Color = activeCol;
            _labelPaint.MaskFilter = _blur;
            _staticCanvas.DrawText(Unit, CENTER_X, CENTER_Y + 130f, SKTextAlign.Center, unitFont, _labelPaint);
            _labelPaint.MaskFilter = null;
            _staticCanvas.DrawText(Unit, CENTER_X, CENTER_Y + 130f, SKTextAlign.Center, unitFont, _labelPaint);

            // Draw title
            using SKFont titleFont = new(_raceFont, 14f);
            _labelPaint.MaskFilter = _blur;
            _staticCanvas.DrawText(Title, CENTER_X, CENTER_Y + 155f, SKTextAlign.Center, titleFont, _labelPaint);
            _labelPaint.MaskFilter = null;
            _staticCanvas.DrawText(Title, CENTER_X, CENTER_Y + 155f, SKTextAlign.Center, titleFont, _labelPaint);

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
                _currentValue += (Value - _currentValue) * 0.1m;
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

            // Draw needle
            float pct = (float)((_currentValue - MinValue) / (MaxValue - MinValue));
            pct = Math.Clamp(pct, 0f, 1f);
            float needleAngle = StartAngleRad + pct * RangeAngleRad;
            float cos = MathF.Cos(needleAngle);
            float sin = MathF.Sin(needleAngle);

            float tipX = CENTER_X + cos * NEEDLE_LENGTH;
            float tipY = CENTER_Y + sin * NEEDLE_LENGTH;
            float tailX = CENTER_X - cos * NEEDLE_TAIL;
            float tailY = CENTER_Y - sin * NEEDLE_TAIL;

            // Perpendicular for needle width
            float perpX = -sin * NEEDLE_HALF_WIDTH;
            float perpY = cos * NEEDLE_HALF_WIDTH;

            using SKPath needlePath = new();
            needlePath.MoveTo(tipX, tipY);
            needlePath.LineTo(CENTER_X + perpX, CENTER_Y + perpY);
            needlePath.LineTo(tailX, tailY);
            needlePath.LineTo(CENTER_X - perpX, CENTER_Y - perpY);
            needlePath.Close();

            // Glow
            _needleGlowPaint.Color = activeCol;
            _needleGlowPaint.MaskFilter = _blur;
            canvas.DrawPath(needlePath, _needleGlowPaint);

            // Crisp needle
            _needleFillPaint.Color = activeCol;
            _needleFillPaint.MaskFilter = null;
            canvas.DrawPath(needlePath, _needleFillPaint);
            _hubPaint.MaskFilter = null;
            canvas.DrawCircle(CENTER_X, CENTER_Y, 6f, _hubPaint);

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
