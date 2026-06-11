using System.Collections.Concurrent;
using SkiaSharp;

namespace GaugeDotnet.Gauges.Custom.Rendering;

internal static class ColorCache
{
    private static readonly ConcurrentDictionary<string, SKColor> _cache =
        new(StringComparer.OrdinalIgnoreCase);

    internal static SKColor Get(string hex) =>
        _cache.GetOrAdd(hex, SKColor.Parse);

    internal static void Clear() => _cache.Clear();
}
