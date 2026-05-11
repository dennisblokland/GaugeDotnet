using SkiaSharp;

namespace GaugeDotnet.Rendering
{
    public static class FontHelper
    {
        public static readonly SKTypeface Default = LoadDefaultTypeface();

        private static SKTypeface LoadDefaultTypeface()
        {
            string bundled = Path.Combine(AppContext.BaseDirectory, "fonts", "BarlowCondensed-Regular.ttf");
            if (File.Exists(bundled))
                return SKTypeface.FromFile(bundled);

            return SKTypeface.Default;
        }

        private static readonly Dictionary<string, string> FontFiles = new()
        {
            { "DSEG14 Classic", "DSEG14Classic-Regular.ttf" },
            { "DSEG7 Classic", "DSEG7Classic-Regular.ttf" },
            { "Race Sport", "Race Sport.ttf" },
            { "Barlow Condensed", "BarlowCondensed-Regular.ttf" }
        };

        private static readonly Dictionary<string, SKTypeface> FontCache = new();

        public static SKTypeface GetFont(string fontKey)
        {
            if (FontCache.TryGetValue(fontKey, out SKTypeface? cachedTypeface))
            {
                return cachedTypeface;
            }

            if (!FontFiles.TryGetValue(fontKey, out string? fontFile))
            {
                throw new ArgumentException($"Font key '{fontKey}' not found in font dictionary.");
            }

            string fontPath = Path.Combine(AppContext.BaseDirectory, "fonts", fontFile);
            if (!File.Exists(fontPath))
            {
                throw new FileNotFoundException($"Font file '{fontFile}' not found in 'fonts' directory.");
            }

            SKTypeface typeface = SKTypeface.FromFile(fontPath);
            FontCache[fontKey] = typeface;
            return typeface;
        }
    }
}
