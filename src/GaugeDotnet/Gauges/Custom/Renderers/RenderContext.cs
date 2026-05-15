using System.Collections.Concurrent;
using GaugeDotnet.Rendering;
using SkiaSharp;

namespace GaugeDotnet.Gauges.Custom.Rendering;

internal static class RenderContext
{
	internal static readonly SKPaint Paint = new() { IsAntialias = true };
	internal static readonly SKFont Font = new(FontHelper.Default);
	private static readonly ConcurrentDictionary<float, SKMaskFilter> _blurCache = new();

	internal static SKMaskFilter GetBlur(float sigma) =>
		_blurCache.GetOrAdd(sigma, s => SKMaskFilter.CreateBlur(SKBlurStyle.Normal, s));

	internal static SKTypeface GetTypeface(string fontKey)
	{
		try { return FontHelper.GetFont(fontKey); }
		catch { return FontHelper.Default; }
	}

	internal static void ClearBlurCache()
	{
		foreach (SKMaskFilter filter in _blurCache.Values)
			filter.Dispose();
		_blurCache.Clear();
	}
}
