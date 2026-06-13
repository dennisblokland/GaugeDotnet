using System.Collections.Concurrent;
using SkiaSharp;

namespace GaugeDotnet.Gauges.Custom.Rendering;

/// <summary>
/// Decodes every frame of a (possibly animated) image into a bitmap array,
/// honouring per-frame blending so partial GIF frames render correctly.
/// </summary>
internal static class GifCache
{
	private static readonly ConcurrentDictionary<string, SKBitmap[]?> _cache = new();

	/// <summary>Returns all decoded frames for the path, or null if it cannot be loaded.</summary>
	internal static SKBitmap[]? Load(string path, string? baseDirectory = null)
	{
		string resolved = Resolve(path, baseDirectory);
		return _cache.GetOrAdd(resolved, Decode);
	}

	private static SKBitmap[]? Decode(string path)
	{
		if (!File.Exists(path)) return null;
		try
		{
			using SKCodec codec = SKCodec.Create(path);
			if (codec == null) return null;

			SKImageInfo info = codec.Info;
			SKCodecFrameInfo[] frameInfos = codec.FrameInfo;
			int count = Math.Max(1, codec.FrameCount);
			SKBitmap[] frames = new SKBitmap[count];

			for (int i = 0; i < count; i++)
			{
				SKBitmap bmp = new(info);
				int required = (frameInfos.Length > i) ? frameInfos[i].RequiredFrame : -1;

				SKCodecOptions options;
				if (required != -1 && frames[required] != null)
				{
					// Decoder blends frame i on top of the prior frame's pixels.
					frames[required].CopyTo(bmp);
					options = new SKCodecOptions(i, required);
				}
				else
				{
					options = new SKCodecOptions(i);
				}

				codec.GetPixels(info, bmp.GetPixels(), options);
				frames[i] = bmp;
			}

			return frames;
		}
		catch { return null; }
	}

	private static string Resolve(string path, string? baseDirectory)
	{
		if (Path.IsPathRooted(path)) return path;
		string? dir = baseDirectory ?? ImageCache.BaseDirectory;
		if (dir != null) return Path.Combine(dir, path);
		return Path.Combine(AppContext.BaseDirectory, path);
	}

	internal static void Clear()
	{
		foreach (SKBitmap[]? frames in _cache.Values)
			if (frames != null)
				foreach (SKBitmap bmp in frames)
					bmp?.Dispose();
		_cache.Clear();
	}
}
