using SkiaSharp;

namespace GaugeDotnet.Extensions
{
    public static class SKCanvasExtensions
    {
        public static void DrawRectWithBlur(this SKCanvas canvas, SKRect rect, SKPaint paint, SKMaskFilter maskFilter)
        {
            SKMaskFilter originalMaskFilter = paint.MaskFilter;

            // Draw the shadow first
            paint.MaskFilter = maskFilter;
            canvas.DrawRect(rect, paint);

            // Restore the original mask filter and draw the actual rectangle
            paint.MaskFilter = originalMaskFilter;
            canvas.DrawRect(rect, paint);
        }

        public static void DrawTextWithBlur(this SKCanvas canvas, string text, float x, float y, SKTextAlign textAlign, SKFont font, SKPaint paint, SKMaskFilter maskFilter)
        {
            SKMaskFilter originalMaskFilter = paint.MaskFilter;

            // Draw the shadow first
            paint.MaskFilter = maskFilter;
            canvas.DrawText(text, x, y, textAlign, font, paint);

            // Restore the original mask filter and draw the actual text
            paint.MaskFilter = originalMaskFilter;
            canvas.DrawText(text, x, y, textAlign, font, paint);
        }
    }
}