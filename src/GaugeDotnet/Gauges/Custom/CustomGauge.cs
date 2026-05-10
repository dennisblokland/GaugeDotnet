using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using System.Text.Json.Serialization;
using GaugeDotnet.Configuration;
using GaugeDotnet.Gauges.Models;
using ME1_4NET;
using SkiaSharp;

namespace GaugeDotnet.Gauges.Custom;

public class CustomGauge : BaseGauge
{
	private const float SmoothingFactor = 0.1f;
	private readonly CustomGaugeDefinition _definition;
	private readonly Dictionary<string, float> _values = new();
	private readonly Dictionary<string, float> _targetValues = new();

	public CustomGauge(CustomGaugeDefinition definition, BaseGaugeSettings settings)
		: base(settings)
	{
		_definition = definition;

		foreach (GaugeElement element in _definition.Elements)
		{
			if (!string.IsNullOrEmpty(element.DataSource) && !_values.ContainsKey(element.DataSource))
			{
				_values[element.DataSource] = 0f;
				_targetValues[element.DataSource] = 0f;
			}
		}
	}

	public override void Draw(SKCanvas canvas)
	{
		foreach (string source in _targetValues.Keys)
		{
			_values[source] += (_targetValues[source] - _values[source]) * SmoothingFactor;
		}

		ElementRenderer.Render(canvas, _definition, _values, _targetValues);
	}

	public void UpdateFromDevice(MEData data)
	{
		foreach (string source in _targetValues.Keys)
		{
			_targetValues[source] = DataSourceMapper.ReadValue(data, source);
		}
	}

	public IReadOnlyCollection<string> DataSources => _values.Keys;

	public static CustomGaugeDefinition LoadDefinition(string path)
	{
		string json = File.ReadAllText(path);
		return JsonSerializer.Deserialize(json, CustomGaugeJsonContext.Default.CustomGaugeDefinition)
			?? new CustomGaugeDefinition();
	}
}

[JsonSourceGenerationOptions(WriteIndented = true)]
[JsonSerializable(typeof(CustomGaugeDefinition))]
internal partial class CustomGaugeJsonContext : JsonSerializerContext
{
}
