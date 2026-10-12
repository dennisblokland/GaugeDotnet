// Renders the same workloads as `gauge bench` in the Rust port, using the
// C# renderer on a CPU raster SKSurface (640x480 RGBA8888), and prints
// ms/frame per screen. Only the draw call is timed, in both harnesses.
using System.Diagnostics;
using System.Text.Json;
using GaugeDotnet.Configuration;
using GaugeDotnet.Gauges;
using GaugeDotnet.Gauges.Custom;
using GaugeDotnet.Gauges.Models;
using SkiaSharp;

string configPath = Path.GetFullPath(args.Length > 0 ? args[0] : "../../rust/examples/gauges.json");
int frames = args.Length > 1 ? int.Parse(args[1]) : 600;
int warmup = args.Length > 2 ? int.Parse(args[2]) : 300;
string configDir = Path.GetDirectoryName(configPath)!;

JsonSerializerOptions opts = new() { Converters = { new TolerantGaugeTypeConverter() } };
AppConfig config = JsonSerializer.Deserialize<AppConfig>(File.ReadAllText(configPath), opts)!;

using SKSurface surface = SKSurface.Create(new SKImageInfo(640, 480, SKColorType.Rgba8888, SKAlphaType.Premul));
SKCanvas canvas = surface.Canvas;

Console.WriteLine($"C# / SkiaSharp {typeof(SKCanvas).Assembly.GetName().Version} (CPU raster), .NET {Environment.Version}");
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
	for (int f = 0; f < warmup; f++) { t += 1.0 / 60; step(t); draw(); }
	surface.Flush();
	long ticks = 0;
	for (int f = 0; f < frames; f++)
	{
		t += 1.0 / 60;
		step(t);
		long s = Stopwatch.GetTimestamp();
		draw();
		ticks += Stopwatch.GetTimestamp() - s;
	}
	double ms = ticks * 1000.0 / Stopwatch.Frequency / frames;
	Console.WriteLine($"{label,-28} {ms,9:F3} {1000 / ms,7:F0}");
}

static BaseGauge Create(GaugeConfig c) => c.Type switch
{
	GaugeType.Circular => new CircularGauge(new CircularGaugeSettings { InitialValue = c.InitialValue, MinValue = c.MinValue, MaxValue = c.MaxValue, Unit = c.Unit, Title = c.Title, Decimals = c.Decimals, SegmentCount = c.SegmentCount, Smoothing = c.Smoothing }),
	GaugeType.Histogram => new HistogramGauge(new HistogramGaugeSettings { InitialValue = c.InitialValue, MinValue = c.MinValue, MaxValue = c.MaxValue, Unit = c.Unit, Title = c.Title, Decimals = c.Decimals, MaxDataPoints = c.MaxDataPoints, IntervalMs = c.IntervalMs }),
	GaugeType.Needle => new NeedleGauge(new NeedleGaugeSettings { InitialValue = c.InitialValue, MinValue = c.MinValue, MaxValue = c.MaxValue, Unit = c.Unit, Title = c.Title, Decimals = c.Decimals, Smoothing = c.Smoothing }),
	GaugeType.Dial => new DialGauge(new DialGaugeSettings { InitialValue = c.InitialValue, MinValue = c.MinValue, MaxValue = c.MaxValue, Unit = c.Unit, Title = c.Title, Decimals = c.Decimals, Smoothing = c.Smoothing }),
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
