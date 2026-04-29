using GaugeDotnet.Extensions;
using SkiaSharp;

namespace GaugeDotnet.Gauges.Components
{
    /// <summary>
    /// Displays a segmented numeric readout (e.g. “1234” style), with a static
    /// background of “~~~~” in an inactive color, and then draws the actual value
    /// in an active color (with a subtle blur‐shadow).
    /// </summary>
    public class SegmentDisplay
    {
        private const string BLANK_DISPLAY = "~~~~";

        private readonly float _centerX;
        private readonly float _centerY;
        private SKColor _inactiveColor;
        private SKColor _activeColor;
        private readonly float _shadowBlur;      // e.g. 15
        private readonly int _decimals;          // how many decimals
        private float _currentValue = 0;
        private string _formattedValue = "0";

        private float? _textWidthCache = null;
        private float _textXcached = 0f;
        private float _baselineY = 0f;

        // Static bitmap/canvas: draw the inactive placeholder once.
        private readonly SKBitmap _staticBitmap;
        private readonly SKCanvas _staticCanvas;

        private readonly SKPaint _textPaint;
        private readonly SKFont _fontFace;
        private bool _staticInitialized = false;
        private SKMaskFilter _blur;

        /// <param name="centerX">x‐center of the text</param>
        /// <param name="centerY">y‐center of the text</param>
        /// <param name="inactiveHex">e.g. “#001919”</param>
        /// <param name="activeHex">e.g. “#00FFFF”</param>
        /// <param name="shadowBlur">blur radius for the active digits</param>
        /// <param name="decimals">number of decimals to show</param>
        public SegmentDisplay(
            int screenWidth,
            int screenHeight,
            float centerX,
            float centerY,
            string inactiveHex,
            string activeHex,
            float shadowBlur = 15f,
            int decimals = 0,
            float fontSize = 70f)
        {
            _centerX = centerX;
            _centerY = centerY;
            _inactiveColor = SKColor.Parse(inactiveHex);
            _activeColor = SKColor.Parse(activeHex);
            _shadowBlur = shadowBlur;
            _decimals = decimals;

            // Create a static SKBitmap + SKCanvas to draw “~~~~” once
            _staticBitmap = new SKBitmap(screenWidth, screenHeight, SKColorType.Rgba8888, SKAlphaType.Premul);
            _staticCanvas = new SKCanvas(_staticBitmap);

            _fontFace = new SKFont(FontHelper.GetFont("DSEG14 Classic"), fontSize);

            // Text paint (we’ll mutate color + blur per draw)
            _textPaint = new SKPaint
            {
                IsAntialias = true,
                IsStroke = false
            };
            _blur = SKMaskFilter.CreateBlur(SKBlurStyle.Outer, _shadowBlur);

            _formattedValue = FormatDisplayValue(_currentValue);
        }

        /// <summary>
        /// Call once if the color(s) changed – resets static cache.
        /// </summary>
        public void SetColors(string inactiveHex, string activeHex)
        {
            SKColor newInactive = SKColor.Parse(inactiveHex);
            SKColor newActive = SKColor.Parse(activeHex);
            if (newInactive == _inactiveColor && newActive == _activeColor)
                return;

            _inactiveColor = newInactive;
            _activeColor = newActive;
            _staticInitialized = false;
        }

        public void SetColors(SKColor inactiveColor, SKColor activeColor)
        {
            if (inactiveColor == _inactiveColor && activeColor == _activeColor)
                return;

            _inactiveColor = inactiveColor;
            _activeColor = activeColor;
            _staticInitialized = false;
        }

        /// <summary>
        /// Update the numeric value (trigger a redraw next time).
        /// </summary>
        public void SetValue(float v)
        {
            const float epsilon = 0.00001f;
            if (Math.Abs(v - _currentValue) < epsilon) return;
            _currentValue = v;
            _formattedValue = FormatDisplayValue(_currentValue);
        }

        /// <summary>
        /// Draw the static background (“~~~~”) in inactive color.
        /// (Only once or when colors change.)
        /// </summary>
        private void DrawStaticBackgroundIfNeeded()
        {
            if (_staticInitialized) return;

            // Clear
            _staticCanvas.Clear(SKColors.Transparent);

            // Configure paint to draw BLANK_DISPLAY in inactive color
            _textPaint.Color = _inactiveColor;
            _textPaint.MaskFilter = null; // no blur

            if (!_textWidthCache.HasValue)
            {
                _textWidthCache = _fontFace.MeasureText(BLANK_DISPLAY);
                _textXcached = _centerX + (_textWidthCache.Value / 2f);
            }

            // Position: the text is right‐aligned, so x = textXcached,
            // y = centerY (but Skia’s DrawText y = baseline, so we need to shift)
            SKFontMetrics fontMetrics = _fontFace.Metrics;
            // Compute baseline offset so that vertical center is at _centerY.
            _baselineY = _centerY + ((fontMetrics.Descent + fontMetrics.Ascent) / 2f) * -1f;

            _staticCanvas.DrawText(
                BLANK_DISPLAY,
                _textXcached,
                _baselineY,
                SKTextAlign.Right,
                _fontFace,
                _textPaint);

            _staticInitialized = true;
        }

        /// <summary>
        /// Call each frame to draw the segment display onto the given canvas.
        /// </summary>
        public void DrawOnCanvas(SKCanvas targetCanvas)
        {
            DrawStaticBackgroundIfNeeded();

            // Draw static background (bitblt)
            targetCanvas.DrawBitmap(_staticBitmap, 0, 0);

            // Now draw actual value over top
            _textPaint.Color = _activeColor;

            float textX = _textXcached; // right‐aligned
            float textY = _baselineY;
            // Apply a simple blur for “shadow”
            // Draw shadow (blurred)

            targetCanvas.DrawTextWithBlur(_formattedValue, textX, textY, SKTextAlign.Right, _fontFace, _textPaint, _blur);

            // Reset blur for safety
            _textPaint.MaskFilter = null;
        }

        private string FormatDisplayValue(float value)
        {
            string s = value.ToString($"F{_decimals}", System.Globalization.CultureInfo.InvariantCulture);
            int dotIndex = s.IndexOf('.');
            string integerPart = dotIndex >= 0 ? s[..dotIndex] : s;
            string decPart = dotIndex >= 0 && dotIndex < s.Length - 1 ? s[(dotIndex + 1)..] : "";

            if (integerPart.Length < 4 && decPart.Length > 0)
            {
                int allowed = 4 - integerPart.Length;
                if (allowed < decPart.Length)
                    decPart = decPart[..allowed];
                return $"{integerPart}.{decPart}";
            }
            else
            {
                return integerPart;
            }
        }
    }
}
