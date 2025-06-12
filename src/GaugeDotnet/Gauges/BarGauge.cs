using GaugeDotnet.Extentions;
using GaugeDotnet.Gauges.Componets;
using GaugeDotnet.Gauges.Models;
using SkiaSharp;

namespace GaugeDotnet.Gauges
{
    /// <summary>
    /// A horizontal bar gauge that:
    /// 1) Caches the “inactive segments + border + unit + title” once,
    /// 2) On each Draw, overlays the active segments and calls SegmentDisplay.
    /// </summary>
    public class BarGauge : BaseGauge
    {
        private decimal _currentValue;     // for smoothing
        private readonly bool _smoothing;
        private readonly int _segmentCount;
        private readonly float _segmentWidth;

        private readonly SegmentDisplay _valueDisplay;
        private readonly SKMaskFilter _blur;

        // Instructions:
        // - staticBitmap/staticCanvas: draw the “inactive bar + border + text”
        // - dynamic drawing happens each frame onto target SKCanvas
        private readonly SKBitmap _staticBitmap;
        private readonly SKCanvas _staticCanvas;

        private readonly float _width;
        private readonly float _height;
        private readonly float _x;
        private readonly float _y;

        private const float SHADOW_BLUR = 15f;

        public BarGauge(
            BarGaugeSettings settings,
            int screenWidth = 800,
            int screenHeight = 480
        ) : base(settings)
        {
            _currentValue = Value;
            _smoothing = settings.Smoothing;
            _segmentCount = settings.SegmentCount;

            // Define bar geometry (hardcoded as in your TS):
            _width = 500f;
            _height = 60f;
            _x = 70f;
            _y = 210f;
            _segmentWidth = _width / _segmentCount;

            // Prepare static cache
            _staticBitmap = new SKBitmap(screenWidth, screenHeight, SKColorType.Rgba8888, SKAlphaType.Premul);
            _staticCanvas = new SKCanvas(_staticBitmap);

            // Prepare embedded SegmentDisplay (positioned at center 320,120)
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
            _blur = SKMaskFilter.CreateBlur(SKBlurStyle.Normal, SHADOW_BLUR);
        }

        /// <summary>
        /// Draw the “inactive” portion (background, inactive segments, border, unit, title) once.
        /// </summary>
        private void UpdateStaticBackground(int screenWidth, int screenHeight)
        {
            DrawBackground(_staticCanvas, screenWidth, screenHeight);

            // Draw inactive segments:
            (SKColor activeCol, SKColor inactiveCol) = Colors;
            using (SKPaint paint = new()
            {
                Style = SKPaintStyle.Fill,
                Color = inactiveCol,
                IsAntialias = true
            })
            {
                for (int i = 0; i < _segmentCount; i++)
                {
                    float segX = _x + i * _segmentWidth;
                    SKRect rect = new(segX + 2f, _y, segX + _segmentWidth - 2f, _y + _height);
                    _staticCanvas.DrawRect(rect, paint);
                }
            }

            // Draw border around the bar:
            using (SKPaint paint = new()
            {
                Style = SKPaintStyle.Stroke,
                Color = activeCol,
                StrokeWidth = 4f,
                IsAntialias = true
            })
            {
                SKRect borderRect = new(
                    _x - 4f,
                    _y - 4f,
                    _x + _width + 4f,
                    _y + _height + 4f
                );
                _staticCanvas.DrawRectWithBlur(borderRect, paint, _blur);

                paint.MaskFilter = null; // reset
            }
            SKTypeface fontFace = FontHelper.GetFont("Race Sport");
            // Draw unit (centered at x=320, y=_y+_height+40)
            using (SKPaint paint = new()
            {
                Color = activeCol,
                IsAntialias = true,
            })
            {
                using SKFont font = new(fontFace, 30f);


                float unitY = _y + _height + 80f;
                _staticCanvas.DrawTextWithBlur(Unit, 320f, unitY, SKTextAlign.Center, font, paint, _blur);
        
                // Draw title (italic) below unit
                font.Size = 20f;
                font.Typeface = fontFace;
                float titleY = unitY + 40f;
                _staticCanvas.DrawTextWithBlur(Title, 320f, titleY, SKTextAlign.Center, font, paint, _blur);
            }

            StaticCacheValid = true;

        }

        /// <summary>
        /// Each frame:
        /// 1) Smooth currentValue toward Value if needed
        /// 2) If static cache invalid, regenerate it
        /// 3) Draw static cache onto target canvasá
        /// 4) Draw active segments
        /// 5) Draw the value via SegmentDisplay
        /// </summary>
        public override void Draw(SKCanvas canvas)
        {
            (SKColor activeCol, SKColor inactiveCol) = Colors;

            // 1) Smooth:
            if (_smoothing)
            {
                _currentValue += (Value - _currentValue) * 0.1m;
            }
            else
            {
                _currentValue = Value;
            }

            // 2) Static cache
            if (!StaticCacheValid)
            {
                UpdateStaticBackground(canvas.DeviceClipBounds.Width, canvas.DeviceClipBounds.Height);
            }

            // 3) Draw static
            canvas.DrawBitmap(_staticBitmap, 0, 0);

            // 4) Draw active segments
            using (SKPaint paint = new()
            {
                Style = SKPaintStyle.Fill,
                Color = activeCol,
                IsAntialias = true
            })
            {
                float pct = (float)((_currentValue - MinValue) / (MaxValue - MinValue));
                pct = SKRectExtensions.Clamp(pct, 0f, 1f);
                int activeSegs = (int)Math.Round(pct * _segmentCount);

                for (int i = 0; i < activeSegs; i++)
                {
                    float segX = _x + i * _segmentWidth;
                    SKRect rect = new(segX + 2f, _y, segX + _segmentWidth - 2f, _y + _height);
                    canvas.DrawRectWithBlur(rect, paint, _blur);
                }
                paint.MaskFilter = null;
            }

            // 5) Draw the numeric value:
            _valueDisplay.SetValue(Value);
            _valueDisplay.SetColors(
                $"#{inactiveCol.Red:X2}{inactiveCol.Green:X2}{inactiveCol.Blue:X2}",
                $"#{activeCol.Red:X2}{activeCol.Green:X2}{activeCol.Blue:X2}"
            );
            _valueDisplay.DrawOnCanvas(canvas);
        }
    }

    /// <summary>
    /// Helper extension to clamp floats.
    /// </summary>
    public static class SKRectExtensions
    {
        public static float Clamp(float val, float min, float max)
        {
            if (val < min) return min;
            if (val > max) return max;
            return val;
        }
    }
}
