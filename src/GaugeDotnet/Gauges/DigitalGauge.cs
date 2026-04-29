using GaugeDotnet.Gauges.Componets;
using GaugeDotnet.Gauges.Models;
using SkiaSharp;

namespace GaugeDotnet.Gauges
{
    /// <summary>
    /// Large centered segment display only — maximizes the numeric readout.
    /// Shows unit/title labels with a subtle border frame.
    /// </summary>
    public class DigitalGauge : BaseGauge
    {
        private readonly SegmentDisplay _valueDisplay;
        private readonly SKMaskFilter _blur;
        private readonly SKTypeface _raceFont;

        private readonly SKBitmap _staticBitmap;
        private readonly SKCanvas _staticCanvas;

        private SKColor _cachedActiveColor;
        private SKColor _cachedInactiveColor;

        private const float SHADOW_BLUR = 15f;
        private const float CENTER_X = 320f;
        private const float CENTER_Y = 220f;

        private readonly SKPaint _borderPaint;
        private readonly SKPaint _labelPaint;

        public DigitalGauge(
            DigitalGaugeSettings settings,
            int screenWidth = 640,
            int screenHeight = 480
        ) : base(settings)
        {
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

            _borderPaint = new SKPaint
            {
                Style = SKPaintStyle.Stroke,
                StrokeWidth = 3f,
                IsAntialias = true,
            };

            _labelPaint = new SKPaint { IsAntialias = true };
        }

        private void UpdateStaticBackground(int screenWidth, int screenHeight)
        {
            DrawBackground(_staticCanvas, screenWidth, screenHeight);

            (SKColor activeCol, SKColor inactiveCol) = Colors;

            // Draw border frame around value area
            _borderPaint.Color = activeCol;
            _borderPaint.MaskFilter = _blur;
            SKRect borderRect = new(80f, CENTER_Y - 60f, screenWidth - 80f, CENTER_Y + 60f);
            _staticCanvas.DrawRoundRect(borderRect, 8f, 8f, _borderPaint);
            _borderPaint.MaskFilter = null;
            _staticCanvas.DrawRoundRect(borderRect, 8f, 8f, _borderPaint);

            // Draw corner accents
            float accentLen = 20f;
            _borderPaint.StrokeWidth = 2f;
            _borderPaint.Color = inactiveCol;
            // Top-left
            _staticCanvas.DrawLine(borderRect.Left - 10f, borderRect.Top - 10f, borderRect.Left - 10f + accentLen, borderRect.Top - 10f, _borderPaint);
            _staticCanvas.DrawLine(borderRect.Left - 10f, borderRect.Top - 10f, borderRect.Left - 10f, borderRect.Top - 10f + accentLen, _borderPaint);
            // Top-right
            _staticCanvas.DrawLine(borderRect.Right + 10f, borderRect.Top - 10f, borderRect.Right + 10f - accentLen, borderRect.Top - 10f, _borderPaint);
            _staticCanvas.DrawLine(borderRect.Right + 10f, borderRect.Top - 10f, borderRect.Right + 10f, borderRect.Top - 10f + accentLen, _borderPaint);
            // Bottom-left
            _staticCanvas.DrawLine(borderRect.Left - 10f, borderRect.Bottom + 10f, borderRect.Left - 10f + accentLen, borderRect.Bottom + 10f, _borderPaint);
            _staticCanvas.DrawLine(borderRect.Left - 10f, borderRect.Bottom + 10f, borderRect.Left - 10f, borderRect.Bottom + 10f - accentLen, _borderPaint);
            // Bottom-right
            _staticCanvas.DrawLine(borderRect.Right + 10f, borderRect.Bottom + 10f, borderRect.Right + 10f - accentLen, borderRect.Bottom + 10f, _borderPaint);
            _staticCanvas.DrawLine(borderRect.Right + 10f, borderRect.Bottom + 10f, borderRect.Right + 10f, borderRect.Bottom + 10f - accentLen, _borderPaint);
            _borderPaint.StrokeWidth = 3f;

            // Unit label
            using SKFont unitFont = new(_raceFont, 30f);
            _labelPaint.Color = activeCol;
            _labelPaint.MaskFilter = _blur;
            _staticCanvas.DrawText(Unit, CENTER_X, CENTER_Y + 110f, SKTextAlign.Center, unitFont, _labelPaint);
            _labelPaint.MaskFilter = null;
            _staticCanvas.DrawText(Unit, CENTER_X, CENTER_Y + 110f, SKTextAlign.Center, unitFont, _labelPaint);

            // Title
            using SKFont titleFont = new(_raceFont, 20f);
            _labelPaint.MaskFilter = _blur;
            _staticCanvas.DrawText(Title, CENTER_X, CENTER_Y + 155f, SKTextAlign.Center, titleFont, _labelPaint);
            _labelPaint.MaskFilter = null;
            _staticCanvas.DrawText(Title, CENTER_X, CENTER_Y + 155f, SKTextAlign.Center, titleFont, _labelPaint);

            // Min/Max range indicators
            using SKFont rangeFont = new(_raceFont, 14f);
            _labelPaint.Color = inactiveCol;
            _staticCanvas.DrawText($"{MinValue}", borderRect.Left + 10f, borderRect.Top - 15f, SKTextAlign.Left, rangeFont, _labelPaint);
            _staticCanvas.DrawText($"{MaxValue}", borderRect.Right - 10f, borderRect.Top - 15f, SKTextAlign.Right, rangeFont, _labelPaint);

            _valueDisplay.SetColors(inactiveCol, activeCol);
            _cachedActiveColor = activeCol;
            _cachedInactiveColor = inactiveCol;

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
