// Renders the same workloads as `gauge bench` in the Rust port, using the
// C# renderer on a CPU raster SKSurface (640x480 RGBA8888), and prints
// ms/frame per screen. Only the draw call is timed, in both harnesses.
// With --gpu it draws through Skia's GLES2 backend in an SDL window, set up as
// the app's GaugeSDL does. Each timed frame then also flushes and waits for the
// GPU to finish, so the GPU's work is counted. Buffers are not swapped, so the
// display's refresh rate doesn't cap the result.
using System.Diagnostics;
using System.Text.Json;
using GaugeDotnet.Configuration;
using GaugeDotnet.Gauges;
using GaugeDotnet.Gauges.Custom;
using GaugeDotnet.Gauges.Models;
using SDL2;
using SkiaSharp;

bool gpu = args.Contains("--gpu");
// --png <dir> saves each screen's last frame, read back from the surface.
int pngArg = Array.IndexOf(args, "--png");
string? pngDir = pngArg >= 0 && pngArg + 1 < args.Length ? args[pngArg + 1] : null;
int pngIndex = 0;
string[] pos = args.Where((a, i) => !a.StartsWith("--") && (pngArg < 0 || i != pngArg + 1)).ToArray();
string configPath = Path.GetFullPath(pos.Length > 0 ? pos[0] : "../../rust/examples/gauges.json");
int frames = pos.Length > 1 ? int.Parse(pos[1]) : 600;
int warmup = pos.Length > 2 ? int.Parse(pos[2]) : 300;
string configDir = Path.GetDirectoryName(configPath)!;

JsonSerializerOptions opts = new() { Converters = { new TolerantGaugeTypeConverter() } };
AppConfig config = JsonSerializer.Deserialize<AppConfig>(File.ReadAllText(configPath), opts)!;

using GpuTarget? gpuTarget = gpu ? new GpuTarget(640, 480) : null;
GRContext? grContext = gpuTarget?.Context;
using SKSurface surface = gpuTarget?.Surface ?? SKSurface.Create(new SKImageInfo(640, 480, SKColorType.Rgba8888, SKAlphaType.Premul));
SKCanvas canvas = surface.Canvas;

string backend = gpuTarget != null ? $"GPU, {gpuTarget.Renderer}" : "CPU raster";
Console.WriteLine($"C# / SkiaSharp {typeof(SKCanvas).Assembly.GetName().Version} ({backend}), .NET {Environment.Version}");
Console.WriteLine($"{"screen",-28} {"ms/frame",9} {"fps",7}");
for (int i = 0; i < config.Screens.Count; i++)
{
	ScreenConfig sc = config.Screens[i];
	GaugeConfig gc = sc.Gauge;
	Action<double> step;
	string label;

	if (gc.Type == GaugeType.Custom && sc.CustomDefinitionFile != null)
	{
		CustomGaugeDefinition def = CustomGauge.LoadDefinition(Path.Combine(configDir, sc.CustomDefinitionFile));
		label = $"Custom ({Path.GetFileName(sc.CustomDefinitionFile)})";
		// Same as CustomGauge.Draw (lerp + ElementRenderer.Render), but fed from the
		// simulator by name: CustomGauge.UpdateFromDevice throws for elements bound to
		// calculated channels, which showcase.json uses.
		Dictionary<string, float> values = new(), targets = new();
		foreach (GaugeElement e in def.Elements)
			if (!string.IsNullOrEmpty(e.DataSource)) { values[e.DataSource] = 0; targets[e.DataSource] = 0; }
		foreach (CalculatedChannel ch in def.CalculatedChannels)
			foreach (string id in new[] { "Rpm", "OilTemp", "Clt" })
				if (ch.Expression.Contains(id)) { values.TryAdd(id, 0); targets.TryAdd(id, 0); }
		float alpha = 1f - MathF.Pow(0.9f, 1f);
		step = t =>
		{
			foreach (string k in targets.Keys) targets[k] = Sim.Read(k, t) ?? targets[k];
			foreach (string k in targets.Keys) values[k] += (targets[k] - values[k]) * alpha;
		};
		Bench(label, step, () => ElementRenderer.Render(canvas, def, values, targets, def.BaseDirectory));
		continue;
	}

	BaseGauge gauge = Create(gc);
	if (gc.Type != GaugeType.Grid) gauge.SetColorHex(gc.ColorHex);
	label = $"{gc.Type} ({(gc.Type == GaugeType.Grid ? "4 cells" : gc.DataSource)})";
	step = t =>
	{
		if (gauge is GridGauge grid)
		{
			for (int c = 0; c < grid.CellCount; c++) grid.SetCellValue(c, Sim.Read(grid.GetCellConfig(c).DataSource, t) ?? 0);
		}
		else gauge.SetValue(Sim.Read(gc.DataSource, t) ?? 0);
	};
	Bench(label, step, () => gauge.Draw(canvas));
}

void Bench(string label, Action<double> step, Action draw)
{
	double t = 0;
	for (int f = 0; f < warmup; f++) { t += 1.0 / 60; step(t); draw(); Finish(); }
	surface.Flush();
	long ticks = 0;
	for (int f = 0; f < frames; f++)
	{
		t += 1.0 / 60;
		step(t);
		long s = Stopwatch.GetTimestamp();
		draw();
		Finish();
		ticks += Stopwatch.GetTimestamp() - s;
	}
	double ms = ticks * 1000.0 / Stopwatch.Frequency / frames;
	Console.WriteLine($"{label,-28} {ms,9:F3} {1000 / ms,7:F0}{(gpu && canvas.Context == null ? "  (no GPU canvas!)" : "")}");
	if (pngDir != null) SavePng(label);
}

void SavePng(string label)
{
	Directory.CreateDirectory(pngDir!);
	using SKBitmap bmp = new(new SKImageInfo(640, 480, SKColorType.Rgba8888, SKAlphaType.Premul));
	if (!surface.ReadPixels(bmp.Info, bmp.GetPixels(), bmp.RowBytes, 0, 0)) { Console.WriteLine($"  read-back failed: {label}"); return; }
	string name = new string(label.Select(c => char.IsLetterOrDigit(c) ? c : '_').ToArray()).Trim('_');
	using FileStream fs = File.Create(Path.Combine(pngDir!, $"{++pngIndex:D2}_{name}.png"));
	bmp.Encode(fs, SKEncodedImageFormat.Png, 100);
}

// On the GPU, Skia only records the draw; this submits it and waits for it.
void Finish() => grContext?.Flush(true, true);

static BaseGauge Create(GaugeConfig c) => c.Type switch
{
	GaugeType.Circular => new CircularGauge(new CircularGaugeSettings { InitialValue = c.InitialValue, MinValue = c.MinValue, MaxValue = c.MaxValue, Unit = c.Unit, Title = c.Title, Decimals = c.Decimals, SegmentCount = c.SegmentCount, Smoothing = c.Smoothing }),
	GaugeType.Histogram => new HistogramGauge(new HistogramGaugeSettings { InitialValue = c.InitialValue, MinValue = c.MinValue, MaxValue = c.MaxValue, Unit = c.Unit, Title = c.Title, Decimals = c.Decimals, MaxDataPoints = c.MaxDataPoints, IntervalMs = c.IntervalMs }),
	GaugeType.Needle => new NeedleGauge(new NeedleGaugeSettings { InitialValue = c.InitialValue, MinValue = c.MinValue, MaxValue = c.MaxValue, Unit = c.Unit, Title = c.Title, Decimals = c.Decimals, Smoothing = c.Smoothing }),
	GaugeType.TurboXs => new TurboXsGauge(new TurboXsGaugeSettings { InitialValue = c.InitialValue, MinValue = c.MinValue, MaxValue = c.MaxValue, Unit = c.Unit, Title = c.Title, Decimals = c.Decimals, Smoothing = c.Smoothing, StatusMode = c.StatusMode, StatusLabel = c.StatusLabel, StatusText = c.StatusText, Lcd = c.Lcd, LcdGhosting = c.LcdGhosting }),
	GaugeType.Digital => new DigitalGauge(new DigitalGaugeSettings { InitialValue = c.InitialValue, MinValue = c.MinValue, MaxValue = c.MaxValue, Unit = c.Unit, Title = c.Title, Decimals = c.Decimals }),
	GaugeType.Sweep => new SweepGauge(new SweepGaugeSettings { InitialValue = c.InitialValue, MinValue = c.MinValue, MaxValue = c.MaxValue, Unit = c.Unit, Title = c.Title, Decimals = c.Decimals, Smoothing = c.Smoothing }),
	GaugeType.MinMax => new MinMaxGauge(new MinMaxGaugeSettings { InitialValue = c.InitialValue, MinValue = c.MinValue, MaxValue = c.MaxValue, Unit = c.Unit, Title = c.Title, Decimals = c.Decimals, SegmentCount = c.SegmentCount, Smoothing = c.Smoothing }),
	GaugeType.Grid => new GridGauge(new GridGaugeSettings { InitialValue = c.InitialValue, MinValue = c.MinValue, MaxValue = c.MaxValue, Unit = c.Unit, Title = c.Title, Cells = c.Cells }),
	_ => new BarGauge(new BarGaugeSettings { InitialValue = c.InitialValue, MinValue = c.MinValue, MaxValue = c.MaxValue, Unit = c.Unit, Title = c.Title, Decimals = c.Decimals, SegmentCount = c.SegmentCount, Smoothing = c.Smoothing }),
};

/// Same formulas as SimulatedMeDevice / rust/gauge-app/src/sim.rs.
static class Sim
{
	public static float? Read(string name, double t)
	{
		double sine = Math.Sin(t), fast = Math.Sin(t * 3);
		double? v = name.ToLowerInvariant() switch
		{
			"rpm" => Math.Truncate(3500 + sine * 3000),
			"throttleposition" or "map" => 50 + sine * 50,
			"iat" => 35 + fast * 10,
			"afrcurr1" => Math.Round(14.5 + sine * 2.5, 2),
			"oiltemp" => 90 + fast * 15,
			"clt" => 85 + fast * 10,
			"vbat" => 13.8 + fast * 0.5,
			"vehiclespeed" => Math.Truncate(60 + sine * 60),
			"boostctrlduty" => 50 + sine * 40,
			_ => null,
		};
		return v is double d ? (float)d : null;
	}
}

/// GLES2 context and window-backed surface, as in GaugeSDL.
sealed class GpuTarget : IDisposable
{
	readonly IntPtr _window, _gl;
	public GRContext Context { get; }
	public SKSurface Surface { get; }
	public string Renderer { get; }

	public GpuTarget(int width, int height)
	{
		if (SDL.SDL_Init(SDL.SDL_INIT_VIDEO) < 0) throw new InvalidOperationException($"SDL_Init: {SDL.SDL_GetError()}");
		SDL.SDL_GL_SetAttribute(SDL.SDL_GLattr.SDL_GL_CONTEXT_PROFILE_MASK, (int)SDL.SDL_GLprofile.SDL_GL_CONTEXT_PROFILE_ES);
		SDL.SDL_GL_SetAttribute(SDL.SDL_GLattr.SDL_GL_CONTEXT_MAJOR_VERSION, 2);
		SDL.SDL_GL_SetAttribute(SDL.SDL_GLattr.SDL_GL_CONTEXT_MINOR_VERSION, 0);
		_window = SDL.SDL_CreateWindow("GaugeBench", SDL.SDL_WINDOWPOS_CENTERED, SDL.SDL_WINDOWPOS_CENTERED, width, height,
			SDL.SDL_WindowFlags.SDL_WINDOW_OPENGL | SDL.SDL_WindowFlags.SDL_WINDOW_SHOWN);
		if (_window == IntPtr.Zero) throw new InvalidOperationException($"SDL_CreateWindow: {SDL.SDL_GetError()}");
		_gl = SDL.SDL_GL_CreateContext(_window);
		if (_gl == IntPtr.Zero) throw new InvalidOperationException($"SDL_GL_CreateContext: {SDL.SDL_GetError()}");
		SDL.SDL_GL_MakeCurrent(_window, _gl);
		SDL.SDL_GL_SetSwapInterval(0);

		GRGlInterface gl = GRGlInterface.Create() ?? GRGlInterface.CreateGles(SDL.SDL_GL_GetProcAddress)
			?? throw new InvalidOperationException("GRGlInterface failed");
		Context = GRContext.CreateGl(gl) ?? throw new InvalidOperationException("GRContext.CreateGl failed");
		Context.SetResourceCacheLimit(16 * 1024 * 1024);
		GRBackendRenderTarget rt = new(width, height, 0, 8, new GRGlFramebufferInfo(0u, SKColorType.Rgba8888.ToGlSizedFormat()));
		Surface = SKSurface.Create(Context, rt, GRSurfaceOrigin.BottomLeft, SKColorType.Rgba8888)
			?? throw new InvalidOperationException("GPU SKSurface failed");
		Renderer = $"{Context.Backend} via SDL {SDL.SDL_GetCurrentVideoDriver()}, {GlString(0x1F01)}";
	}

	// glGetString, for the GL_RENDERER name.
	delegate IntPtr GlGetString(uint name);
	static string GlString(uint name)
	{
		IntPtr fn = SDL.SDL_GL_GetProcAddress("glGetString");
		if (fn == IntPtr.Zero) return "?";
		IntPtr s = System.Runtime.InteropServices.Marshal.GetDelegateForFunctionPointer<GlGetString>(fn)(name);
		return System.Runtime.InteropServices.Marshal.PtrToStringAnsi(s) ?? "?";
	}

	public void Dispose()
	{
		Surface.Dispose();
		Context.Dispose();
		SDL.SDL_GL_DeleteContext(_gl);
		SDL.SDL_DestroyWindow(_window);
		SDL.SDL_Quit();
	}
}
