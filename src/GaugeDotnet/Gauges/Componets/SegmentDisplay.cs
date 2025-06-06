using GaugeDotnet.Extentions;
using SkiaSharp;
using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace GaugeDotnet.Gauges.Componets
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
        private decimal _currentValue = 0;

        private float? _textWidthCache = null;
        private float _textXcached = 0f;

        // Two bitmaps/canvases: one for static background, one for dynamic value
        private readonly SKBitmap _staticBitmap;
        private readonly SKCanvas _staticCanvas;
        private readonly SKBitmap _dynamicBitmap;
        private readonly SKCanvas _dynamicCanvas;

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
            int decimals = 0)
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

            // Create a dynamic SKBitmap + SKCanvas to draw “actual value” each frame
            _dynamicBitmap = new SKBitmap(screenWidth, screenHeight, SKColorType.Rgba8888, SKAlphaType.Premul);
            _dynamicCanvas = new SKCanvas(_dynamicBitmap);

            _fontFace = new SKFont(FontHelper.GetFont("DSEG14 Classic"), 70f);

            // Text paint (we’ll mutate color + blur per draw)
            _textPaint = new SKPaint
            {
                IsAntialias = true,
                IsStroke = false
            };
            _blur = SKMaskFilter.CreateBlur(SKBlurStyle.Outer, _shadowBlur);
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

        /// <summary>
        /// Update the numeric value (trigger a redraw next time).
        /// </summary>
        public void SetValue(decimal v)
        {
            const decimal epsilon = 0.00001M;
            if (Math.Abs(v - _currentValue) < epsilon) return;
            _currentValue = v;
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
            // Compute baseline offset so that vertical center is at _centerY
            float textHeight = fontMetrics.Descent - fontMetrics.Ascent;
            float baselineY = _centerY + ((fontMetrics.Descent + fontMetrics.Ascent) / 2f) * -1f;

            _staticCanvas.DrawText(
                BLANK_DISPLAY,
                _textXcached,
                baselineY,
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

            // Clear dynamic canvas
            _dynamicCanvas.Clear(SKColors.Transparent);

            // Draw static background (bitblt)
            _dynamicCanvas.DrawBitmap(_staticBitmap, 0, 0);

            // Now draw actual value over top
            string txt = FormatDisplayValue();
            _textPaint.Color = _activeColor;



            float textX = _textXcached; // right‐aligned
            float textY;
            {
                SKFontMetrics fm = _fontFace.Metrics;
                textY = _centerY + (fm.Descent + fm.Ascent) / 2f * -1f;
            }
            // Apply a simple blur for “shadow”
            // Draw shadow (blurred)

            _dynamicCanvas.DrawTextWithBlur(txt, textX, textY, SKTextAlign.Right, _fontFace, _textPaint, _blur);

            // Reset blur for safety
            _textPaint.MaskFilter = null;

            // Finally blit the dynamic bitmap onto the target canvas
            targetCanvas.DrawBitmap(_dynamicBitmap, 0, 0);
        }

        private string FormatDisplayValue()
        {
            string s = _currentValue.ToString($"F{_decimals}", System.Globalization.CultureInfo.InvariantCulture);
            string[] parts = s.Split('.');
            string integerPart = parts[0];
            string decPart = parts.Length > 1 ? parts[1] : "";

            if (integerPart.Length < 4 && decPart.Length > 0)
            {
                int allowed = 4 - integerPart.Length;
                if (allowed < decPart.Length)
                    decPart = decPart.Substring(0, allowed);
                return $"{integerPart}.{decPart}";
            }
            else
            {
                return integerPart;
            }
        }
    }
}
