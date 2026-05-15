using System;
using System.Collections.Concurrent;
using System.IO;
using SkiaSharp;

namespace GaugeDotnet.Gauges.Custom.Rendering;

internal static class ImageCache
{
	private static readonly ConcurrentDictionary<string, SKBitmap?> _cache = new();
	internal static string? BaseDirectory { get; set; }

	internal static SKBitmap? Load(string path, string? baseDirectory = null)
	{
		string resolved = Resolve(path, baseDirectory);
		return _cache.GetOrAdd(resolved, p =>
		{
			if (!File.Exists(p)) return null;
			try { return SKBitmap.Decode(p); }
			catch { return null; }
		});
	}

	private static string Resolve(string path, string? baseDirectory)
	{
		if (Path.IsPathRooted(path)) return path;
		string? dir = baseDirectory ?? BaseDirectory;
		if (dir != null) return Path.Combine(dir, path);
		return Path.Combine(AppContext.BaseDirectory, path);
	}

	internal static bool SaveToDisk(string imagePath, string destPath)
	{
		SKBitmap? bitmap = Load(imagePath, BaseDirectory);
		if (bitmap == null) return false;
		using SKData data = bitmap.Encode(SKEncodedImageFormat.Png, 100);
		using FileStream fs = File.Create(destPath);
		data.SaveTo(fs);
		return true;
	}

	internal static void Clear()
	{
		foreach (SKBitmap? bitmap in _cache.Values)
			bitmap?.Dispose();
		_cache.Clear();
	}
}
