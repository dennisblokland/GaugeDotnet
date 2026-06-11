using GaugeDotnet.Configuration;
using SkiaSharp;

namespace GaugeDotnet.Rendering
{
    public class BatteryOverlay
    {
        private readonly AppConfig _config;
        private readonly int _screenWidth;
        private readonly SKPaint _paint = new() { IsAntialias = true };

        private static readonly SKColor ColorWarning = new(0xFF, 0x8C, 0x00); // orange
        private static readonly SKColor ColorCritical = new(0xFF, 0x20, 0x20); // red
        private static readonly SKColor ColorOutline = new(0xCC, 0xCC, 0xCC);

        // Cache battery reads – sysfs is cheap but no need to hammer every frame
        private int? _cachedPercent;
        private DateTime _lastRead = DateTime.MinValue;

        public BatteryOverlay(AppConfig config, int screenWidth)
        {
            _config = config;
            _screenWidth = screenWidth;
        }

        private int? GetBatteryPercent()
        {
            if (_config.SimulateBatteryPercent.HasValue)
            {
                return _config.SimulateBatteryPercent.Value;
            }

            if ((DateTime.UtcNow - _lastRead).TotalSeconds < 30)
            {
                return _cachedPercent;
            }

            _lastRead = DateTime.UtcNow;
            _cachedPercent = ReadSystemBattery();
            return _cachedPercent;
        }

        private static int? ReadSystemBattery()
        {
            string[] candidates =
            [
                "/sys/class/power_supply/axp20x-battery/capacity",
                "/sys/class/power_supply/BAT0/capacity",
                "/sys/class/power_supply/BAT1/capacity",
                "/sys/class/power_supply/battery/capacity",
            ];

            foreach (string path in candidates)
            {
                if (!File.Exists(path))
                {
                    continue;
                }

                string text = File.ReadAllText(path).Trim();
                if (int.TryParse(text, out int pct))
                {
                    return Math.Clamp(pct, 0, 100);
                }
            }

            return null;
        }

        public void Draw(SKCanvas canvas)
        {
            int? pct = GetBatteryPercent();

            if (pct == null || pct > 30)
            {
                return;
            }

            bool critical = pct <= 15;

            // Blink when critical
            if (critical && DateTime.Now.Millisecond > 500)
            {
                return;
            }

            SKColor fillColor = critical ? ColorCritical : ColorWarning;

            // Icon geometry: body 48x20, nub 5x10, positioned top-right
            const float bodyW = 48f;
            const float bodyH = 20f;
            const float nubW = 5f;
            const float nubH = 10f;
            const float padding = 12f;

            float x = _screenWidth - bodyW - nubW - padding;
            float y = 8f;

            float fillW = (bodyW - 4f) * (pct.Value / 100f);

            // Body outline
            _paint.Style = SKPaintStyle.Stroke;
            _paint.StrokeWidth = 1.5f;
            _paint.Color = ColorOutline;
            canvas.DrawRoundRect(x, y, bodyW, bodyH, 3, 3, _paint);

            // Terminal nub
            _paint.Style = SKPaintStyle.Fill;
            _paint.Color = ColorOutline;
            canvas.DrawRoundRect(x + bodyW, y + (bodyH - nubH) / 2f, nubW, nubH, 2, 2, _paint);

            // Fill bar
            if (fillW > 0)
            {
                _paint.Color = fillColor;
                canvas.DrawRect(x + 2f, y + 2f, fillW, bodyH - 4f, _paint);
            }

            // Percentage label inside body
            using SKFont font = new(FontHelper.Default) { Size = 11 };
            _paint.Color = SKColors.White;
            _paint.Style = SKPaintStyle.Fill;
            string label = $"{pct}%";
            canvas.DrawText(label, x + bodyW / 2f, y + bodyH / 2f + 4f, SKTextAlign.Center, font, _paint);
        }

        public void Dispose()
        {
            _paint.Dispose();
        }
    }
}
