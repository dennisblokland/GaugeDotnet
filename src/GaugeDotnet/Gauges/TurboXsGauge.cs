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
    /// With <see cref="TurboXsGaugeSettings.Lcd"/> it imitates an old backlit LCD: rendered at
    /// a third of the resolution with a visible pixel grid, a blue tint and ghosting.
    /// </summary>
    public class TurboXsGauge : BaseGauge
    {
        private float _currentValue;
        private readonly bool _smoothing;
        private readonly int _decimals;
        private readonly int _majorCount;

        private enum StatusMode { Peak, Min, Text, None }
        private readonly StatusMode _statusMode;
        private readonly string _statusLabel;
        private readonly string _statusText;
        private float _peak;
        private float _min;

        private readonly SKTypeface _font;
        private readonly SKTypeface _digitFont;
        private readonly SKBitmap _staticBitmap;
        private readonly SKCanvas _staticCanvas;

        // Drawing coordinates are always 640x480-style; _renderScale shrinks them
        // to LCD resolution in Lcd mode (1 otherwise).
        private readonly bool _lcd;
        private readonly float _renderScale;
        private readonly float _lcdGhosting;
        private readonly int _width;
        private readonly int _height;
        private SKSurface? _lcdFrame;
        private bool _lcdPrimed;
        // GPU canvas: ghosted image as a surface, and the grid as a repeating tile shader.
        private SKSurface? _lcdAccum;
        private SKShader? _lcdGrid;
        // CPU canvas: ghosted RGB per LCD pixel, and the full-size output bitmap.
        private byte[]? _lcdAccumPixels;
        private SKBitmap? _lcdOut;
        private readonly SKPaint _lcdBlendPaint;
        private readonly SKPaint _lcdPaint = new();

        private const float CENTER_X = 221f;
        private const float CENTER_Y = 243f;
        private const float BEZEL_RADIUS = 207f;
        private const float BEZEL_WIDTH = 17f;
        private const float BEZEL_OVERHANG_DEG = 18f;
        private const float BEZEL_FADE_DEG = 15f;
        private const float BEZEL_START_DEG = START_ANGLE_DEG - BEZEL_OVERHANG_DEG;
        private const float BEZEL_SWEEP_DEG = SWEEP_DEG + 2f * BEZEL_OVERHANG_DEG;
        private const float START_ANGLE_DEG = 90f;
        private const float SWEEP_DEG = 270f;
        private const int MINOR_PER_MAJOR = 5;
        private const float TICK_OUTER = 178f;
        private const float TICK_INNER = 157f;
        private const float DOT_RADIUS = 170f;
        private const float LABEL_RADIUS = 124f;
        private const float NEEDLE_LENGTH = 148f;
        private const float NEEDLE_HALF_WIDTH = 5.5f;
        private const float RIGHT_EDGE = 612f;

        // Old LCD: each LCD pixel covers LCD_CELL x LCD_CELL screen pixels; the gap
        // between them is darkened to LCD_GRID; each frame keeps _lcdGhosting of the
        // last one (TurboXsGaugeSettings.LcdGhosting); and black/white map to the backlight's navy and pale blue.
        private const int LCD_CELL = 3;
        private const float LCD_GRID = 0.7f;
        private static readonly SKColor LcdBlack = new(0x14, 0x1A, 0x40);
        private static readonly SKColor LcdWhite = new(0xD8, 0xE0, 0xFF);
        // CPU path: per channel (R, G, B), the tinted value and the tinted value on a grid line.
        private static readonly (byte[] Plain, byte[] Line)[] LcdTables = BuildLcdTables();

        // Widest each text may be before it shrinks, so long values, titles and
        // labels stay clear of the dial whatever the data source and range.
        private const float VALUE_MAX_WIDTH = 180f;
        private const float STATUS_MAX_WIDTH = 175f;
        private const float UNIT_MAX_WIDTH = 220f;
        private const float LABEL_MAX_WIDTH = 56f;
        private static readonly float[] TitleMaxWidth = [270f, 225f];

        private static readonly SKColor BezelColor = new(0xA8, 0xA8, 0xB4);
        private static readonly SKColor BezelHighlight = new(0xE8, 0xE8, 0xF0);
        private static readonly SKColor NeedleColor = new(0xF0, 0x28, 0x48);
        private static readonly SKColor TextColor = SKColors.White;
        private static readonly SKColor StatusColor = new(0xD8, 0xDA, 0xEC);
        private static readonly SKColor StatusDotColor = new(0x30, 0x60, 0xFF);

        private readonly SKPaint _fillPaint;
        private readonly SKPaint _boldTextPaint;
        private readonly SKPaint _strokePaint;

        public TurboXsGauge(
            TurboXsGaugeSettings settings,
            int screenWidth = 640,
            int screenHeight = 480
        ) : base(settings)
        {
            _currentValue = Value;
            _smoothing = settings.Smoothing;
            _decimals = Math.Max(0, settings.Decimals);
            _statusMode = Enum.TryParse(settings.StatusMode, true, out StatusMode mode) ? mode : StatusMode.Peak;
            _statusLabel = !string.IsNullOrEmpty(settings.StatusLabel) ? settings.StatusLabel : _statusMode switch
            {
                StatusMode.Min => "Min:",
                StatusMode.Text => "Datalog:",
                _ => "Peak:",
            };
            _statusText = settings.StatusText ?? "";
            _majorCount = MajorIntervals(MinValue, MaxValue);
            // Peak and Min start from the first reading, not the initial value.
            _peak = float.NaN;
            _min = float.NaN;

            _lcd = settings.Lcd;
            _renderScale = _lcd ? 1f / LCD_CELL : 1f;
            _lcdGhosting = float.IsFinite(settings.LcdGhosting) ? Math.Clamp(settings.LcdGhosting, 0f, 0.9f) : 0f;
            _width = screenWidth;
            _height = screenHeight;
            (int renderW, int renderH) = _lcd ? LcdSize() : (screenWidth, screenHeight);
            _lcdBlendPaint = new SKPaint { ColorFilter = SKColorFilter.CreateColorMatrix(LcdTintMatrix()) };

            _staticBitmap = new SKBitmap(renderW, renderH, SKColorType.Rgba8888, SKAlphaType.Premul);
            _staticCanvas = new SKCanvas(_staticBitmap);
            _font = FontHelper.GetFont("Aldrich");
            _digitFont = FontHelper.GetFont("Saira Extra Condensed");

            _fillPaint = new SKPaint { Style = SKPaintStyle.Fill, IsAntialias = true };
            _boldTextPaint = new SKPaint { Style = SKPaintStyle.StrokeAndFill, IsAntialias = true };
            _strokePaint = new SKPaint { Style = SKPaintStyle.Stroke, IsAntialias = true };
        }

        private (int Width, int Height) LcdSize()
        {
            return ((_width + LCD_CELL - 1) / LCD_CELL, (_height + LCD_CELL - 1) / LCD_CELL);
        }

        /// <summary>Maps black to <see cref="LcdBlack"/> and white to <see cref="LcdWhite"/>, per channel.</summary>
        private static float[] LcdTintMatrix()
        {
            static float Scale(byte black, byte white) => (white - black) / 255f;
            return
            [
                Scale(LcdBlack.Red, LcdWhite.Red), 0, 0, 0, LcdBlack.Red / 255f,
                0, Scale(LcdBlack.Green, LcdWhite.Green), 0, 0, LcdBlack.Green / 255f,
                0, 0, Scale(LcdBlack.Blue, LcdWhite.Blue), 0, LcdBlack.Blue / 255f,
                0, 0, 0, 1, 0,
            ];
        }

        private static float AngleFor(float fraction)
        {
            return (START_ANGLE_DEG + fraction * SWEEP_DEG) * MathF.PI / 180f;
        }

        /// <summary>
        /// Number of major intervals (4 to 8, preferring 6) that puts every major tick
        /// on a round value: a step of 1, 2, 2.5 or 5 times a power of ten.
        /// </summary>
        private static int MajorIntervals(float min, float max)
        {
            float range = max - min;
            if (!(range > 0f))
            {
                return 6;
            }
            foreach (int n in new[] { 6, 5, 7, 4, 8 })
            {
                float step = range / n;
                float magnitude = MathF.Pow(10f, MathF.Floor(MathF.Log10(step)));
                float mantissa = step / magnitude;
                foreach (float nice in new[] { 1f, 2f, 2.5f, 5f, 10f })
                {
                    if (MathF.Abs(mantissa - nice) < 1e-3f)
                    {
                        return n;
                    }
                }
            }
            return 6;
        }

        /// <summary>Tick label: up to two decimals, without trailing zeros or "-0".</summary>
        private static string FormatLabel(float v)
        {
            float rounded = MathF.Round(v * 100f) / 100f;
            return (rounded == 0f ? 0f : rounded).ToString("0.##", CultureInfo.InvariantCulture);
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
                canvas.Scale(_renderScale);
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
                    float fade = BezelFade((x + 0.5f) / _renderScale, (y + 0.5f) / _renderScale);
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
        /// Text wider than <paramref name="maxWidth"/> is shrunk to fit around (x, y).
        /// </summary>
        private static void DrawStyledText(
            SKCanvas canvas, string text, float x, float y, SKTextAlign align,
            SKFont font, SKPaint paint, float scaleX, float skew, float spacing,
            float maxWidth = float.PositiveInfinity)
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

            float fit = Math.Min(1f, maxWidth / (width * scaleX));

            canvas.Save();
            canvas.Translate(x, y);
            canvas.Scale(fit, fit);
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

        private void UpdateStaticBackground()
        {
            DrawBackground(_staticCanvas, _staticBitmap.Width, _staticBitmap.Height);
            DrawBezel(_staticCanvas, _staticBitmap.Width, _staticBitmap.Height);

            _staticCanvas.Save();
            _staticCanvas.Scale(_renderScale);

            // Major ticks and labels.
            _strokePaint.Color = TextColor;
            _strokePaint.StrokeWidth = 4f;
            _boldTextPaint.Color = TextColor;
            _boldTextPaint.StrokeWidth = 1f;
            using SKFont labelFont = new(_font, 36f);
            for (int i = 0; i <= _majorCount; i++)
            {
                float a = AngleFor((float)i / _majorCount);
                float cos = MathF.Cos(a);
                float sin = MathF.Sin(a);
                _staticCanvas.DrawLine(
                    CENTER_X + cos * TICK_INNER, CENTER_Y + sin * TICK_INNER,
                    CENTER_X + cos * TICK_OUTER, CENTER_Y + sin * TICK_OUTER,
                    _strokePaint);

                float tickValue = MinValue + ((float)i / _majorCount) * (MaxValue - MinValue);
                string label = FormatLabel(tickValue);
                DrawStyledText(_staticCanvas, label,
                    CENTER_X + cos * LABEL_RADIUS, CENTER_Y + sin * LABEL_RADIUS + 13f,
                    SKTextAlign.Center, labelFont, _boldTextPaint, 1f, 0f, 0f, LABEL_MAX_WIDTH);
            }

            // Minor dots.
            _fillPaint.Color = TextColor;
            int minorCount = _majorCount * MINOR_PER_MAJOR;
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
                    SKTextAlign.Right, titleFont, _boldTextPaint, 1.05f, -0.2f, 7f, TitleMaxWidth[i]);
            }

            // Status block: the label, plus the fixed text and its dot in Text mode.
            // Peak and Min values change, so Draw adds them each frame.
            if (_statusMode != StatusMode.None)
            {
                using SKFont statusFont = new(_font, 26f);
                _boldTextPaint.Color = StatusColor;
                _boldTextPaint.StrokeWidth = 0f;
                DrawStyledText(_staticCanvas, _statusLabel, RIGHT_EDGE, 222f,
                    SKTextAlign.Right, statusFont, _boldTextPaint, 1f, 0f, 6f, STATUS_MAX_WIDTH);
            }
            if (_statusMode == StatusMode.Text)
            {
                using SKFont statusFont = new(_font, 26f);
                DrawStyledText(_staticCanvas, _statusText, RIGHT_EDGE - 22f, 246f,
                    SKTextAlign.Right, statusFont, _boldTextPaint, 1f, 0f, 6f, STATUS_MAX_WIDTH - 22f);
                _fillPaint.Color = StatusDotColor;
                _staticCanvas.DrawCircle(RIGHT_EDGE - 7f, 238f, 6f, _fillPaint);
            }

            // Unit below the readout.
            _boldTextPaint.Color = TextColor;
            _boldTextPaint.StrokeWidth = 1.4f;
            using SKFont unitFont = new(_font, 44f);
            DrawStyledText(_staticCanvas, Unit, RIGHT_EDGE - 4f, 452f,
                SKTextAlign.Right, unitFont, _boldTextPaint, 1.05f, -0.2f, 7f, UNIT_MAX_WIDTH);

            _staticCanvas.Restore();
            StaticCacheValid = true;
        }

        public override void SetValue(float v)
        {
            base.SetValue(v);
            _peak = float.IsNaN(_peak) ? v : Math.Max(_peak, v);
            _min = float.IsNaN(_min) ? v : Math.Min(_min, v);
        }

        public override void ResetSavedState()
        {
            _peak = Value;
            _min = Value;
            _lcdPrimed = false;
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
                UpdateStaticBackground();
            }

            if (_lcd)
            {
                DrawLcd(canvas);
            }
            else
            {
                DrawFrame(canvas);
            }
        }

        /// <summary>
        /// Renders the frame at LCD resolution, blends it over the previous one (ghosting),
        /// then shows it scaled up with hard pixel edges, the backlight tint and the pixel grid.
        /// A GPU canvas does that in one shader draw. On a CPU canvas, Skia's shader blits are
        /// slow, so it is done like the Rust renderer: lookup tables and row copies into a bitmap.
        /// </summary>
        private void DrawLcd(SKCanvas canvas)
        {
            (int lw, int lh) = LcdSize();
            GRRecordingContext? context = canvas.Context;
            if (_lcdFrame == null)
            {
                SKImageInfo info = new(lw, lh, SKColorType.Rgba8888, SKAlphaType.Premul);
                _lcdFrame = context != null ? SKSurface.Create(context, false, info) : SKSurface.Create(info);
                _lcdPrimed = false;
            }

            _lcdFrame.Canvas.Clear(SKColors.Black);
            DrawFrame(_lcdFrame.Canvas);
            if (context != null)
            {
                ComposeLcdGpu(canvas, context, lw, lh);
            }
            else
            {
                ComposeLcdRaster(canvas, lw, lh);
            }
            _lcdPrimed = true;
        }

        /// <summary>
        /// Blends the frame, tinted, into the ghosted image (the tint is linear, so tinting before
        /// the blend matches tinting after it), then draws it scaled up and multiplied by a
        /// repeating grid tile in a single full-screen draw.
        /// </summary>
        private void ComposeLcdGpu(SKCanvas canvas, GRRecordingContext context, int lw, int lh)
        {
            if (_lcdAccum == null || _lcdGrid == null)
            {
                _lcdAccum = SKSurface.Create(context, false, new SKImageInfo(lw, lh, SKColorType.Rgba8888, SKAlphaType.Premul));
                _lcdGrid = CreateLcdGrid();
            }

            using (SKImage frame = _lcdFrame!.Snapshot())
            {
                byte alpha = _lcdPrimed ? (byte)MathF.Round(255f * (1f - _lcdGhosting)) : (byte)255;
                _lcdBlendPaint.Color = SKColors.White.WithAlpha(alpha);
                _lcdAccum.Canvas.DrawImage(frame, 0, 0, _lcdBlendPaint);
            }

            using SKImage lcd = _lcdAccum.Snapshot();
            using SKShader pixels = lcd.ToShader(SKShaderTileMode.Clamp, SKShaderTileMode.Clamp,
                new SKSamplingOptions(SKFilterMode.Nearest), SKMatrix.CreateScale(LCD_CELL, LCD_CELL));
            using SKShader shader = SKShader.CreateCompose(pixels, _lcdGrid, SKBlendMode.Modulate);
            _lcdPaint.Shader = shader;
            canvas.DrawRect(0, 0, lw * LCD_CELL, lh * LCD_CELL, _lcdPaint);
            _lcdPaint.Shader = null;
        }

        /// <summary>
        /// Same result on the CPU, with the Rust renderer's arithmetic: blend the frame into the
        /// ghosted RGB buffer, then build each LCD row once (a plain row and a grid row) through
        /// the tint tables and copy it into the screen rows it covers.
        /// </summary>
        private void ComposeLcdRaster(SKCanvas canvas, int lw, int lh)
        {
            _lcdAccumPixels ??= new byte[lw * lh * 3];
            _lcdOut ??= new SKBitmap(_width, _height, SKColorType.Rgba8888, SKAlphaType.Premul);

            float alpha = _lcdPrimed ? MathF.Round(255f * (1f - _lcdGhosting)) / 255f : 1f;
            using (SKPixmap frame = _lcdFrame!.PeekPixels())
            {
                ReadOnlySpan<byte> src = frame.GetPixelSpan();
                for (int ly = 0; ly < lh; ly++)
                {
                    ReadOnlySpan<byte> row = src.Slice(ly * frame.RowBytes, lw * 4);
                    for (int lx = 0; lx < lw; lx++)
                    {
                        for (int c = 0; c < 3; c++)
                        {
                            ref byte acc = ref _lcdAccumPixels[(ly * lw + lx) * 3 + c];
                            acc = (byte)MathF.Round(row[lx * 4 + c] * alpha + acc * (1f - alpha), MidpointRounding.AwayFromZero);
                        }
                    }
                }
            }

            (byte[] rPlain, byte[] rLine) = LcdTables[0];
            (byte[] gPlain, byte[] gLine) = LcdTables[1];
            (byte[] bPlain, byte[] bLine) = LcdTables[2];
            Span<byte> dst = _lcdOut.GetPixelSpan();
            Span<uint> plainRow = stackalloc uint[_width];
            Span<uint> lineRow = stackalloc uint[_width];
            for (int ly = 0; ly < lh && ly * LCD_CELL < _height; ly++)
            {
                for (int x = 0; x < _width; x++)
                {
                    int i = (ly * lw + x / LCD_CELL) * 3;
                    byte r = _lcdAccumPixels[i], g = _lcdAccumPixels[i + 1], b = _lcdAccumPixels[i + 2];
                    uint line = rLine[r] | (uint)gLine[g] << 8 | (uint)bLine[b] << 16 | 0xFF000000u;
                    lineRow[x] = line;
                    plainRow[x] = x % LCD_CELL == LCD_CELL - 1
                        ? line
                        : rPlain[r] | (uint)gPlain[g] << 8 | (uint)bPlain[b] << 16 | 0xFF000000u;
                }
                for (int y = ly * LCD_CELL; y < Math.Min((ly + 1) * LCD_CELL, _height); y++)
                {
                    Span<uint> rowPixels = y % LCD_CELL == LCD_CELL - 1 ? lineRow : plainRow;
                    System.Runtime.InteropServices.MemoryMarshal.AsBytes(rowPixels).CopyTo(dst.Slice(y * _width * 4, _width * 4));
                }
            }
            canvas.DrawBitmap(_lcdOut, 0, 0);
        }

        private static (byte[] Plain, byte[] Line)[] BuildLcdTables()
        {
            float grid = 1f - MathF.Round(255f * (1f - LCD_GRID)) / 255f;
            (byte Black, byte White)[] channels =
            [
                (LcdBlack.Red, LcdWhite.Red), (LcdBlack.Green, LcdWhite.Green), (LcdBlack.Blue, LcdWhite.Blue),
            ];
            return channels.Select(ch =>
            {
                byte[] plain = new byte[256], line = new byte[256];
                for (int v = 0; v < 256; v++)
                {
                    float tinted = ch.Black + (ch.White - ch.Black) * v / 255f;
                    plain[v] = (byte)MathF.Round(tinted, MidpointRounding.AwayFromZero);
                    line[v] = (byte)MathF.Round(tinted * grid, MidpointRounding.AwayFromZero);
                }
                return (plain, line);
            }).ToArray();
        }

        /// <summary>
        /// Repeating LCD_CELL x LCD_CELL tile: white, with the last row and column at
        /// <see cref="LCD_GRID"/> brightness. Multiplying by it draws the gaps between LCD pixels.
        /// </summary>
        private static SKShader CreateLcdGrid()
        {
            using SKBitmap tile = new(LCD_CELL, LCD_CELL, SKColorType.Rgba8888, SKAlphaType.Premul);
            byte shade = (byte)MathF.Round(255f * LCD_GRID);
            for (int y = 0; y < LCD_CELL; y++)
            {
                for (int x = 0; x < LCD_CELL; x++)
                {
                    byte v = x == LCD_CELL - 1 || y == LCD_CELL - 1 ? shade : (byte)255;
                    tile.SetPixel(x, y, new SKColor(v, v, v));
                }
            }
            return tile.ToShader(SKShaderTileMode.Repeat, SKShaderTileMode.Repeat);
        }

        /// <summary>The static layer plus needle, readout and status, at <see cref="_renderScale"/>.</summary>
        private void DrawFrame(SKCanvas canvas)
        {
            canvas.DrawBitmap(_staticBitmap, 0, 0);
            canvas.Save();
            canvas.Scale(_renderScale);

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
            DrawStyledText(canvas, text, RIGHT_EDGE, 408f, SKTextAlign.Right, valueFont, _boldTextPaint, 0.85f, -0.12f, 2f, VALUE_MAX_WIDTH);

            if (_statusMode is StatusMode.Peak or StatusMode.Min)
            {
                float held = _statusMode == StatusMode.Peak ? _peak : _min;
                if (float.IsNaN(held))
                {
                    held = Value;
                }
                using SKFont statusFont = new(_font, 26f);
                _boldTextPaint.Color = StatusColor;
                _boldTextPaint.StrokeWidth = 0f;
                DrawStyledText(canvas, held.ToString("F" + _decimals, CultureInfo.InvariantCulture), RIGHT_EDGE, 246f,
                    SKTextAlign.Right, statusFont, _boldTextPaint, 1f, 0f, 6f, STATUS_MAX_WIDTH);
            }
            canvas.Restore();
        }
    }
}
