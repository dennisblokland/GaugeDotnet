using System;
using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace GaugeDotnet.Gauges.Custom;

public class CalculatedChannel
{
	public string Name { get; set; } = "";
	public string Expression { get; set; } = "";
}

public enum BackgroundImageMode
{
	Stretch,
	Fill,
	Fit,
	Center,
	Tile,
}

public class CustomGaugeDefinition
{
	public int Width { get; set; } = 640;
	public int Height { get; set; } = 480;
	public string BackgroundColor { get; set; } = "#000000";
	public string? BackgroundImage { get; set; }
	public BackgroundImageMode BackgroundImageMode { get; set; } = BackgroundImageMode.Stretch;
	public byte BackgroundImageOpacity { get; set; } = 255;
	public List<GaugeElement> Elements { get; set; } = new();
	public List<CalculatedChannel> CalculatedChannels { get; set; } = new();

	[JsonIgnore]
	public string? BaseDirectory { get; set; }
}

[JsonPolymorphic(TypeDiscriminatorPropertyName = "$type")]
[JsonDerivedType(typeof(ArcElement), "arc")]
[JsonDerivedType(typeof(NeedleElement), "needle")]
[JsonDerivedType(typeof(TextElement), "text")]
[JsonDerivedType(typeof(ValueDisplayElement), "value")]
[JsonDerivedType(typeof(TickRingElement), "ticks")]
[JsonDerivedType(typeof(CircleElement), "circle")]
[JsonDerivedType(typeof(RectangleElement), "rectangle")]
[JsonDerivedType(typeof(LineElement), "line")]
[JsonDerivedType(typeof(LinearBarElement), "linearbar")]
[JsonDerivedType(typeof(WarningIndicatorElement), "warning")]
[JsonDerivedType(typeof(ImageElement), "image")]
[JsonDerivedType(typeof(ZoneArcElement), "zonearc")]
[JsonDerivedType(typeof(GraphElement), "graph")]
[JsonDerivedType(typeof(LabelValueElement), "labelvalue")]
[JsonDerivedType(typeof(PeakMarkerElement), "peak")]
public abstract class GaugeElement
{
	public string Id { get; set; } = Guid.NewGuid().ToString("N")[..8];
	public string Name { get; set; } = "";
	public float X { get; set; } = 320f;
	public float Y { get; set; } = 240f;
	public string? DataSource { get; set; }
	public float MinValue { get; set; }
	public float MaxValue { get; set; } = 100f;
	public byte Opacity { get; set; } = 255;
	public bool UseVisibility { get; set; } = false;
	public string? VisibilitySource { get; set; }
	public float VisibleAbove { get; set; } = float.MinValue;
	public float VisibleBelow { get; set; } = float.MaxValue;

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
	public bool AntiClockwise { get; set; } = false;
	public bool UseConditionalColor { get; set; } = false;
	public float WarnThreshold { get; set; } = 80f;
	public string WarnColor { get; set; } = "#FFCC00";
	public float DangerThreshold { get; set; } = 95f;
	public string DangerColor { get; set; } = "#FF3333";

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
	public bool AntiClockwise { get; set; } = false;
	public string Color { get; set; } = "#FF3333";
	public bool ShowHub { get; set; } = true;
	public float HubRadius { get; set; } = 10f;
	public string HubColor { get; set; } = "#CCCCCC";
	public string? ImagePath { get; set; }
	public float ImageWidth { get; set; } = 20f;
	public float ImageLength { get; set; } = 180f;

	[JsonIgnore]
	public override string TypeLabel => "Needle";
}

public class TextElement : GaugeElement
{
	public string Text { get; set; } = "Label";
	public float FontSize { get; set; } = 24f;
	public string Color { get; set; } = "#FFFFFF";
	public string Font { get; set; } = "Race Sport";
	public bool ShowBox { get; set; } = false;
	public string BoxColor { get; set; } = "#1A1A1A";
	public float BoxPadding { get; set; } = 6f;
	public float BoxCornerRadius { get; set; } = 4f;

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
	public bool AntiClockwise { get; set; } = false;
	public int MajorCount { get; set; } = 8;
	public int MinorPerMajor { get; set; } = 4;
	public float MajorLength { get; set; } = 20f;
	public float MinorLength { get; set; } = 10f;
	public float MajorWidth { get; set; } = 3f;
	public float MinorWidth { get; set; } = 1f;
	public string Color { get; set; } = "#FFFFFF";
	public bool ShowTicks { get; set; } = true;
	public bool TicksInside { get; set; }
	public bool ShowLabels { get; set; } = true;
	public bool RadialLabels { get; set; }
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

public class RectangleElement : GaugeElement
{
	public float Width { get; set; } = 100f;
	public float Height { get; set; } = 60f;
	public string FillColor { get; set; } = "#222222";
	public string StrokeColor { get; set; } = "#666666";
	public float RectStrokeWidth { get; set; } = 1f;
	public float CornerRadius { get; set; }

	[JsonIgnore]
	public override string TypeLabel => "Rect";
}

public class LineElement : GaugeElement
{
	public float X2 { get; set; } = 420f;
	public float Y2 { get; set; } = 240f;
	public float LineWidth { get; set; } = 2f;
	public string Color { get; set; } = "#666666";

	[JsonIgnore]
	public override string TypeLabel => "Line";
}

public class LinearBarElement : GaugeElement
{
	public float Width { get; set; } = 200f;
	public float Height { get; set; } = 20f;
	public bool IsVertical { get; set; }
	public string FillColor { get; set; } = "#00FFFF";
	public string TrackColor { get; set; } = "#0A1A1A";
	public string BorderColor { get; set; } = "#444444";
	public float BorderWidth { get; set; } = 1f;
	public float CornerRadius { get; set; } = 2f;
	public bool UseConditionalColor { get; set; } = false;
	public float WarnThreshold { get; set; } = 80f;
	public string WarnColor { get; set; } = "#FFCC00";
	public float DangerThreshold { get; set; } = 95f;
	public string DangerColor { get; set; } = "#FF3333";

	[JsonIgnore]
	public override string TypeLabel => "Bar";
}

public class WarningIndicatorElement : GaugeElement
{
	public float Radius { get; set; } = 12f;
	public float Threshold { get; set; } = 80f;
	public bool TriggerAbove { get; set; } = true;
	public string ActiveColor { get; set; } = "#FF3333";
	public string InactiveColor { get; set; } = "#331111";
	public string Label { get; set; } = "WARN";
	public float LabelFontSize { get; set; } = 12f;
	public string LabelColor { get; set; } = "#FFFFFF";
	public bool ShowLabel { get; set; } = true;

	[JsonIgnore]
	public override string TypeLabel => "Warning";
}

public class ImageElement : GaugeElement
{
	public string ImagePath { get; set; } = "";
	public float Width { get; set; } = 100f;
	public float Height { get; set; } = 100f;
	public float Rotation { get; set; }

	[JsonIgnore]
	public override string TypeLabel => "Image";
}

public class LabelValueElement : GaugeElement
{
	public string Label { get; set; } = "LABEL";
	public float LabelFontSize { get; set; } = 14f;
	public string LabelColor { get; set; } = "#888888";
	public string LabelFont { get; set; } = "Race Sport";
	public float ValueFontSize { get; set; } = 48f;
	public string ValueColor { get; set; } = "#00FFFF";
	public string ValueFont { get; set; } = "DSEG7 Classic";
	public string ValueFormat { get; set; } = "F0";
	public string ValueSuffix { get; set; } = "";
	public bool ShowBox { get; set; } = false;
	public string BoxColor { get; set; } = "#1A1A1A";
	public float BoxPadding { get; set; } = 8f;
	public float BoxCornerRadius { get; set; } = 4f;

	[JsonIgnore]
	public override string TypeLabel => "LabelValue";
}

public class ZoneArcElement : GaugeElement
{
	public float Radius { get; set; } = 200f;
	public float StrokeWidth { get; set; } = 30f;
	public float StartAngleDeg { get; set; } = 135f;
	public float SweepAngleDeg { get; set; } = 270f;
	public bool AntiClockwise { get; set; } = false;
	public string Zone1Color { get; set; } = "#00CC00";
	public float Zone2Start { get; set; } = 70f;
	public string Zone2Color { get; set; } = "#FFCC00";
	public bool ShowZone2 { get; set; } = true;
	public float Zone3Start { get; set; } = 90f;
	public string Zone3Color { get; set; } = "#FF3333";
	public bool ShowZone3 { get; set; } = true;
	public bool ShowPointer { get; set; } = false;
	public string PointerColor { get; set; } = "#FFFFFF";
	public float PointerWidth { get; set; } = 3f;

	[JsonIgnore]
	public override string TypeLabel => "ZoneArc";
}

public class GraphElement : GaugeElement
{
	public float Width { get; set; } = 200f;
	public float Height { get; set; } = 100f;
	public int HistoryDepth { get; set; } = 60;
	public string LineColor { get; set; } = "#00FFFF";
	public string FillColor { get; set; } = "#00FFFF";
	public byte FillOpacity { get; set; } = 40;
	public string BackColor { get; set; } = "#0A1A1A";
	public float LineWidth { get; set; } = 2f;
	public bool ShowFill { get; set; } = true;

	[JsonIgnore]
	public override string TypeLabel => "Graph";
}

public class PeakMarkerElement : GaugeElement
{
	public float Radius { get; set; } = 200f;
	public float StrokeWidth { get; set; } = 30f;
	public float StartAngleDeg { get; set; } = 135f;
	public float SweepAngleDeg { get; set; } = 270f;
	public bool AntiClockwise { get; set; } = false;
	public string MarkerColor { get; set; } = "#FFFFFF";
	public float MarkerWidth { get; set; } = 3f;
	public float DecaySeconds { get; set; } = 0f;

	[JsonIgnore]
	public override string TypeLabel => "Peak";
}
