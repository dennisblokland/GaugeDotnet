using System.Globalization;
using GaugeDotnet.Rendering;
using GaugeDotnet.Gauges.Models;
using SkiaSharp;

namespace GaugeDotnet.Gauges
{
    /// <summary>
    /// Handheld-style boost display: a 270° dial on the left (bezel, white ticks,
    /// red needle) and a two-line title, status text and a large italic digital
    /// readout on the right. Title words are stacked ("Boost Press." → two lines).
    /// </summary>
    public class DialGauge : BaseGauge
    {
        private float _currentValue;
        private readonly bool _smoothing;
        private readonly int _decimals;

        private readonly SKTypeface _font;
        private readonly SKTypeface _digitFont;
        private readonly SKBitmap _staticBitmap;
        private readonly SKCanvas _staticCanvas;

        private const float CENTER_X = 221f;
        private const float CENTER_Y = 243f;
        private const float BEZEL_RADIUS = 207f;
        private const float BEZEL_WIDTH = 17f;
        private const float BEZEL_OVERHANG_DEG = 25f;
        private const float BEZEL_FADE_DEG = 20f;
        private const float BEZEL_START_DEG = START_ANGLE_DEG - BEZEL_OVERHANG_DEG;
        private const float BEZEL_SWEEP_DEG = SWEEP_DEG + 2f * BEZEL_OVERHANG_DEG;
        private const float START_ANGLE_DEG = 90f;
        private const float SWEEP_DEG = 270f;
        private const int MAJOR_COUNT = 6;
        private const int MINOR_PER_MAJOR = 5;
        private const float TICK_OUTER = 178f;
        private const float TICK_INNER = 157f;
        private const float DOT_RADIUS = 170f;
        private const float LABEL_RADIUS = 124f;
        private const float NEEDLE_LENGTH = 148f;
        private const float NEEDLE_HALF_WIDTH = 5.5f;
        private const float RIGHT_EDGE = 612f;

        private static readonly SKColor BezelColor = new(0xA8, 0xA8, 0xB4);
        private static readonly SKColor BezelHighlight = new(0xE8, 0xE8, 0xF0);
        private static readonly SKColor NeedleColor = new(0xF0, 0x28, 0x48);
        private static readonly SKColor TextColor = SKColors.White;
        private static readonly SKColor StatusColor = new(0xD8, 0xDA, 0xEC);
        private static readonly SKColor StatusDotColor = new(0x30, 0x60, 0xFF);

        private readonly SKPaint _fillPaint;
        private readonly SKPaint _boldTextPaint;
        private readonly SKPaint _strokePaint;

        public DialGauge(
            DialGaugeSettings settings,
            int screenWidth = 640,
            int screenHeight = 480
        ) : base(settings)
        {
            _currentValue = Value;
            _smoothing = settings.Smoothing;
            _decimals = Math.Max(0, settings.Decimals);

            _staticBitmap = new SKBitmap(screenWidth, screenHeight, SKColorType.Rgba8888, SKAlphaType.Premul);
            _staticCanvas = new SKCanvas(_staticBitmap);
            _font = FontHelper.GetFont("Aldrich");
            _digitFont = FontHelper.GetFont("Saira Extra Condensed");

            _fillPaint = new SKPaint { Style = SKPaintStyle.Fill, IsAntialias = true };
            _boldTextPaint = new SKPaint { Style = SKPaintStyle.StrokeAndFill, IsAntialias = true };
            _strokePaint = new SKPaint { Style = SKPaintStyle.Stroke, IsAntialias = true };
        }

        private static float AngleFor(float fraction)
        {
            return (START_ANGLE_DEG + fraction * SWEEP_DEG) * MathF.PI / 180f;
        }

        private static float SmoothStep(float t)
        {
            t = Math.Clamp(t, 0f, 1f);
            return t * t * (3f - 2f * t);
        }

        /// <summary>
        /// Bezel opacity at a point: full along the scale, fading out over <see cref="BEZEL_FADE_DEG"/>
        /// at both ends of the bezel, which runs <see cref="BEZEL_OVERHANG_DEG"/> past the end ticks.
        /// </summary>
        private static float BezelFade(float x, float y)
        {
            float deg = MathF.Atan2(y - CENTER_Y, x - CENTER_X) * 180f / MathF.PI;
            float d = (((deg - BEZEL_START_DEG) % 360f) + 360f) % 360f;
            if (d > BEZEL_SWEEP_DEG)
            {
                return 0f;
            }
            return SmoothStep(d / BEZEL_FADE_DEG) * SmoothStep((BEZEL_SWEEP_DEG - d) / BEZEL_FADE_DEG);
        }

        /// <summary>
        /// Grey ring with a lighter outer rim along the scale and a little past it. It is drawn into its own
        /// layer whose alpha is then scaled per pixel by <see cref="BezelFade"/>.
        /// </summary>
        private void DrawBezel(SKCanvas target, int width, int height)
        {
            using SKBitmap layer = new(width, height, SKColorType.Rgba8888, SKAlphaType.Premul);
            using (SKCanvas canvas = new(layer))
            {
                canvas.Clear(SKColors.Transparent);
                float bodyR = BEZEL_RADIUS - BEZEL_WIDTH / 2f;
                _strokePaint.Color = BezelColor;
                _strokePaint.StrokeWidth = BEZEL_WIDTH;
                canvas.DrawArc(new SKRect(CENTER_X - bodyR, CENTER_Y - bodyR, CENTER_X + bodyR, CENTER_Y + bodyR),
                    BEZEL_START_DEG, BEZEL_SWEEP_DEG, false, _strokePaint);
                float rimR = BEZEL_RADIUS - BEZEL_WIDTH * 0.2f;
                _strokePaint.Color = BezelHighlight;
                _strokePaint.StrokeWidth = BEZEL_WIDTH * 0.4f;
                canvas.DrawArc(new SKRect(CENTER_X - rimR, CENTER_Y - rimR, CENTER_X + rimR, CENTER_Y + rimR),
                    BEZEL_START_DEG, BEZEL_SWEEP_DEG, false, _strokePaint);
            }

            Span<byte> pixels = layer.GetPixelSpan();
            for (int y = 0; y < height; y++)
            {
                for (int x = 0; x < width; x++)
                {
                    int i = (y * width + x) * 4;
                    if (pixels[i + 3] == 0)
                    {
                        continue;
                    }
                    float fade = BezelFade(x + 0.5f, y + 0.5f);
                    for (int c = 0; c < 4; c++)
                    {
                        pixels[i + c] = (byte)(pixels[i + c] * fade + 0.5f);
                    }
                }
            }
            target.DrawBitmap(layer, 0, 0);
        }

        /// <summary>
        /// Draws text stretched by <paramref name="scaleX"/>, slanted by <paramref name="skew"/>
        /// and with extra <paramref name="spacing"/> between letters; y is the baseline.
        /// </summary>
        private static void DrawStyledText(
            SKCanvas canvas, string text, float x, float y, SKTextAlign align,
            SKFont font, SKPaint paint, float scaleX, float skew, float spacing)
        {
            if (string.IsNullOrEmpty(text))
            {
                return;
            }

            float width = spacing * (text.Length - 1);
            foreach (char c in text)
            {
                width += font.MeasureText(c.ToString());
            }

            float pen = align switch
            {
                SKTextAlign.Center => -width / 2f,
                SKTextAlign.Right => -width,
                _ => 0f,
            };

            canvas.Save();
            canvas.Translate(x, y);
            canvas.Skew(skew, 0f);
            canvas.Scale(scaleX, 1f);
            foreach (char c in text)
            {
                string s = c.ToString();
                canvas.DrawText(s, pen, 0f, SKTextAlign.Left, font, paint);
                pen += font.MeasureText(s) + spacing;
            }
            canvas.Restore();
        }

        private void UpdateStaticBackground(int screenWidth, int screenHeight)
        {
            DrawBackground(_staticCanvas, screenWidth, screenHeight);

            DrawBezel(_staticCanvas, screenWidth, screenHeight);

            // Major ticks and labels.
            _strokePaint.Color = TextColor;
            _strokePaint.StrokeWidth = 4f;
            _boldTextPaint.Color = TextColor;
            _boldTextPaint.StrokeWidth = 1f;
            using SKFont labelFont = new(_font, 36f);
            for (int i = 0; i <= MAJOR_COUNT; i++)
            {
                float a = AngleFor((float)i / MAJOR_COUNT);
                float cos = MathF.Cos(a);
                float sin = MathF.Sin(a);
                _staticCanvas.DrawLine(
                    CENTER_X + cos * TICK_INNER, CENTER_Y + sin * TICK_INNER,
                    CENTER_X + cos * TICK_OUTER, CENTER_Y + sin * TICK_OUTER,
                    _strokePaint);

                float tickValue = MinValue + ((float)i / MAJOR_COUNT) * (MaxValue - MinValue);
                string label = $"{Math.Round(tickValue)}";
                DrawStyledText(_staticCanvas, label,
                    CENTER_X + cos * LABEL_RADIUS, CENTER_Y + sin * LABEL_RADIUS + 13f,
                    SKTextAlign.Center, labelFont, _boldTextPaint, 1f, 0f, 0f);
            }

            // Minor dots.
            _fillPaint.Color = TextColor;
            int minorCount = MAJOR_COUNT * MINOR_PER_MAJOR;
            for (int i = 0; i <= minorCount; i++)
            {
                if (i % MINOR_PER_MAJOR == 0)
                {
                    continue;
                }
                float a = AngleFor((float)i / minorCount);
                _staticCanvas.DrawCircle(
                    CENTER_X + MathF.Cos(a) * DOT_RADIUS, CENTER_Y + MathF.Sin(a) * DOT_RADIUS, 2.2f, _fillPaint);
            }

            // Title, one word per line, right aligned.
            using SKFont titleFont = new(_font, 50f);
            _boldTextPaint.StrokeWidth = 1.4f;
            string[] words = Title.Split(' ', StringSplitOptions.RemoveEmptyEntries);
            for (int i = 0; i < Math.Min(words.Length, 2); i++)
            {
                DrawStyledText(_staticCanvas, words[i], RIGHT_EDGE, 64f + i * 46f,
                    SKTextAlign.Right, titleFont, _boldTextPaint, 1.05f, -0.2f, 7f);
            }

            // Status block.
            using SKFont statusFont = new(_font, 26f);
            _boldTextPaint.Color = StatusColor;
            _boldTextPaint.StrokeWidth = 0f;
            DrawStyledText(_staticCanvas, "Datalog:", RIGHT_EDGE, 222f,
                SKTextAlign.Right, statusFont, _boldTextPaint, 1f, 0f, 6f);
            DrawStyledText(_staticCanvas, "Stop", RIGHT_EDGE - 22f, 246f,
                SKTextAlign.Right, statusFont, _boldTextPaint, 1f, 0f, 6f);
            _fillPaint.Color = StatusDotColor;
            _staticCanvas.DrawCircle(RIGHT_EDGE - 7f, 238f, 6f, _fillPaint);

            // Unit below the readout.
            _boldTextPaint.Color = TextColor;
            _boldTextPaint.StrokeWidth = 1.4f;
            using SKFont unitFont = new(_font, 44f);
            DrawStyledText(_staticCanvas, Unit, RIGHT_EDGE - 4f, 452f,
                SKTextAlign.Right, unitFont, _boldTextPaint, 1.05f, -0.2f, 7f);

            StaticCacheValid = true;
        }

        public override void Draw(SKCanvas canvas)
        {
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

            // Needle: flat bar from the centre towards the ticks.
            float pct = Math.Clamp((_currentValue - MinValue) / (MaxValue - MinValue), 0f, 1f);
            float a = AngleFor(pct);
            float cos = MathF.Cos(a);
            float sin = MathF.Sin(a);
            float perpX = -sin * NEEDLE_HALF_WIDTH;
            float perpY = cos * NEEDLE_HALF_WIDTH;
            float tipX = CENTER_X + cos * NEEDLE_LENGTH;
            float tipY = CENTER_Y + sin * NEEDLE_LENGTH;

            using SKPath needle = new();
            needle.MoveTo(CENTER_X + perpX, CENTER_Y + perpY);
            needle.LineTo(tipX + perpX, tipY + perpY);
            needle.LineTo(tipX - perpX, tipY - perpY);
            needle.LineTo(CENTER_X - perpX, CENTER_Y - perpY);
            needle.Close();
            _fillPaint.Color = NeedleColor;
            canvas.DrawPath(needle, _fillPaint);

            // Digital readout.
            string text = Value.ToString("F" + _decimals, CultureInfo.InvariantCulture);
            using SKFont valueFont = new(_digitFont, 190f);
            _boldTextPaint.Color = TextColor;
            _boldTextPaint.StrokeWidth = 1f;
            DrawStyledText(canvas, text, RIGHT_EDGE, 408f, SKTextAlign.Right, valueFont, _boldTextPaint, 0.85f, -0.12f, 2f);
        }
    }
}
