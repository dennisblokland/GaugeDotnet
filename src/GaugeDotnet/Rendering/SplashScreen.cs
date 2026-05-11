using SkiaSharp;
using System.Diagnostics;
using static SDL2.SDL;

namespace GaugeDotnet.Rendering
{
    public static class SplashScreen
    {
        private const int Width = 640;
        private const int Height = 480;
        private const double MinDurationSeconds = 1.5;

        public static void ShowUntil(GaugeSDL sdl, Task task, string title = "GaugeDotnet", string subtitle = "Motorsport Telemetry")
        {
            SKTypeface face = FontHelper.GetFont("Race Sport");
            using SKFont titleFont = new(face, 64);
            using SKFont subtitleFont = new(face, 22);
            using SKFont statusFont = new(face, 18);

            using SKPaint titlePaint = new() { Color = SKColors.White, IsAntialias = true };
            using SKPaint accentPaint = new() { Color = new SKColor(0xFF, 0x33, 0x33), IsAntialias = true };
            using SKPaint subtitlePaint = new() { Color = new SKColor(0xCC, 0xCC, 0xCC), IsAntialias = true };
            using SKPaint statusPaint = new() { Color = new SKColor(0x99, 0x99, 0x99), IsAntialias = true };
            using SKPaint barBgPaint = new() { Color = new SKColor(0x22, 0x22, 0x22), IsAntialias = true };
            using SKPaint barFillPaint = new() { Color = new SKColor(0xFF, 0x33, 0x33), IsAntialias = true };

            Stopwatch sw = Stopwatch.StartNew();

            while (true)
            {
                double t = sw.Elapsed.TotalSeconds;

                if (task.IsCompleted && t >= MinDurationSeconds)
                {
                    break;
                }

                while (SDL_PollEvent(out SDL_Event e) == 1)
                {
                    // drain event queue; ignore input during splash
                }

                Render(sdl, t, task.IsCompleted, title, subtitle,
                    titleFont, subtitleFont, statusFont,
                    titlePaint, accentPaint, subtitlePaint, statusPaint,
                    barBgPaint, barFillPaint);

                Thread.Sleep(16);
            }
        }

        private static void Render(
            GaugeSDL sdl, double t, bool taskDone,
            string title, string subtitle,
            SKFont titleFont, SKFont subtitleFont, SKFont statusFont,
            SKPaint titlePaint, SKPaint accentPaint, SKPaint subtitlePaint, SKPaint statusPaint,
            SKPaint barBgPaint, SKPaint barFillPaint)
        {
            SKCanvas canvas = sdl.GetCanvas();

            using (SKPaint bg = new())
            {
                SKPoint top = new(0, 0);
                SKPoint bot = new(0, Height);
                bg.Shader = SKShader.CreateLinearGradient(
                    top, bot,
                    new[] { new SKColor(0x10, 0x10, 0x14), new SKColor(0x00, 0x00, 0x00) },
                    null, SKShaderTileMode.Clamp);
                canvas.DrawRect(0, 0, Width, Height, bg);
            }

            float fadeIn = (float)Math.Clamp(t / 0.4, 0.0, 1.0);
            byte a = (byte)(255 * fadeIn);

            titlePaint.Color = SKColors.White.WithAlpha(a);
            accentPaint.Color = new SKColor(0xFF, 0x33, 0x33).WithAlpha(a);
            subtitlePaint.Color = new SKColor(0xCC, 0xCC, 0xCC).WithAlpha(a);
            statusPaint.Color = new SKColor(0x99, 0x99, 0x99).WithAlpha(a);
            barBgPaint.Color = new SKColor(0x22, 0x22, 0x22).WithAlpha(a);
            barFillPaint.Color = new SKColor(0xFF, 0x33, 0x33).WithAlpha(a);

            float titleW = titleFont.MeasureText(title);
            float titleX = (Width - titleW) / 2f;
            float titleY = 210f;

            float underlineW = titleW + 40f;
            float underlineX = (Width - underlineW) / 2f;
            canvas.DrawRect(underlineX, titleY + 14, underlineW, 4, accentPaint);
            canvas.DrawText(title, titleX, titleY, titleFont, titlePaint);

            float subW = subtitleFont.MeasureText(subtitle);
            canvas.DrawText(subtitle, (Width - subW) / 2f, titleY + 56, subtitleFont, subtitlePaint);

            const float barW = 260f;
            const float barH = 6f;
            float barX = (Width - barW) / 2f;
            float barY = 360f;
            canvas.DrawRoundRect(barX, barY, barW, barH, 3, 3, barBgPaint);
            // Indeterminate sweeping segment while task pending; full bar once done.
            if (taskDone)
            {
                canvas.DrawRoundRect(barX, barY, barW, barH, 3, 3, barFillPaint);
            }
            else
            {
                const float segW = 80f;
                float cycle = (float)(t % 1.6 / 1.6);
                float segX = barX + (barW + segW) * cycle - segW;
                float drawX = Math.Max(barX, segX);
                float drawEnd = Math.Min(barX + barW, segX + segW);
                float drawW = Math.Max(0, drawEnd - drawX);
                if (drawW > 0)
                {
                    canvas.DrawRoundRect(drawX, barY, drawW, barH, 3, 3, barFillPaint);
                }
            }

            int dots = ((int)(t * 3)) % 4;
            string status = taskDone ? "Ready" : "Connecting" + new string('.', dots);
            float statusW = statusFont.MeasureText(status);
            canvas.DrawText(status, (Width - statusW) / 2f, barY + 36, statusFont, statusPaint);

            sdl.FlushAndSwap();
        }
    }
}
