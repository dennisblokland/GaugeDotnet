using SkiaSharp;

namespace GaugeDotnet.Extentions
{
    public static class SKCanvasExtentions
    {
        public static void DrawRectWithBlur(this SKCanvas canvas, SKRect rect, SKPaint paint, SKMaskFilter maskFilter)
        {
            // Draw the shadow first
            paint.MaskFilter = maskFilter;
            canvas.DrawRect(rect, paint);

            // Reset the mask filter for the actual rectangle
            paint.MaskFilter = null;
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