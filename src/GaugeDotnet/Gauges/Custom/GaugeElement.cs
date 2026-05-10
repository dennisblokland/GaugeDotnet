using System;
using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace GaugeDotnet.Gauges.Custom;

public class CustomGaugeDefinition
{
	public int Width { get; set; } = 640;
	public int Height { get; set; } = 480;
	public string BackgroundColor { get; set; } = "#000000";
	public List<GaugeElement> Elements { get; set; } = new();
}

[JsonPolymorphic(TypeDiscriminatorPropertyName = "$type")]
[JsonDerivedType(typeof(ArcElement), "arc")]
[JsonDerivedType(typeof(NeedleElement), "needle")]
[JsonDerivedType(typeof(TextElement), "text")]
[JsonDerivedType(typeof(ValueDisplayElement), "value")]
[JsonDerivedType(typeof(TickRingElement), "ticks")]
[JsonDerivedType(typeof(CircleElement), "circle")]
public abstract class GaugeElement
{
	public string Id { get; set; } = Guid.NewGuid().ToString("N")[..8];
	public string Name { get; set; } = "";
	public float X { get; set; } = 320f;
	public float Y { get; set; } = 240f;
	public string? DataSource { get; set; }
	public float MinValue { get; set; }
	public float MaxValue { get; set; } = 100f;

	[JsonIgnore]
	public abstract string TypeLabel { get; }
}

public class ArcElement : GaugeElement
{
	public float Radius { get; set; } = 200f;
	public float StrokeWidth { get; set; } = 30f;
	public float StartAngleDeg { get; set; } = 135f;
	public float SweepAngleDeg { get; set; } = 270f;
	public string Color { get; set; } = "#00FFFF";
	public string TrackColor { get; set; } = "#0A1A1A";
	public bool ShowTrack { get; set; } = true;
	public bool IsDynamic { get; set; } = true;

	[JsonIgnore]
	public override string TypeLabel => "Arc";
}

public class NeedleElement : GaugeElement
{
	public float Length { get; set; } = 180f;
	public float TailLength { get; set; } = 25f;
	public float NeedleWidth { get; set; } = 4f;
	public float StartAngleDeg { get; set; } = 135f;
	public float SweepAngleDeg { get; set; } = 270f;
	public string Color { get; set; } = "#FF3333";
	public bool ShowHub { get; set; } = true;
	public float HubRadius { get; set; } = 10f;
	public string HubColor { get; set; } = "#CCCCCC";

	[JsonIgnore]
	public override string TypeLabel => "Needle";
}

public class TextElement : GaugeElement
{
	public string Text { get; set; } = "Label";
	public float FontSize { get; set; } = 24f;
	public string Color { get; set; } = "#FFFFFF";
	public string Font { get; set; } = "Race Sport";

	[JsonIgnore]
	public override string TypeLabel => "Text";
}

public class ValueDisplayElement : GaugeElement
{
	public float FontSize { get; set; } = 48f;
	public string Color { get; set; } = "#00FFFF";
	public string Font { get; set; } = "DSEG7 Classic";
	public string Format { get; set; } = "F0";
	public string Suffix { get; set; } = "";

	[JsonIgnore]
	public override string TypeLabel => "Value";
}

public class TickRingElement : GaugeElement
{
	public float Radius { get; set; } = 200f;
	public float StartAngleDeg { get; set; } = 135f;
	public float SweepAngleDeg { get; set; } = 270f;
	public int MajorCount { get; set; } = 8;
	public int MinorPerMajor { get; set; } = 4;
	public float MajorLength { get; set; } = 20f;
	public float MinorLength { get; set; } = 10f;
	public float MajorWidth { get; set; } = 3f;
	public float MinorWidth { get; set; } = 1f;
	public string Color { get; set; } = "#FFFFFF";
	public bool ShowLabels { get; set; } = true;
	public float LabelFontSize { get; set; } = 14f;
	public string LabelColor { get; set; } = "#CCCCCC";
	public float LabelOffset { get; set; } = 25f;

	[JsonIgnore]
	public override string TypeLabel => "Ticks";
}

public class CircleElement : GaugeElement
{
	public float Radius { get; set; } = 20f;
	public string FillColor { get; set; } = "#333333";
	public string StrokeColor { get; set; } = "#666666";
	public float CircleStrokeWidth { get; set; } = 2f;

	[JsonIgnore]
	public override string TypeLabel => "Circle";
}
