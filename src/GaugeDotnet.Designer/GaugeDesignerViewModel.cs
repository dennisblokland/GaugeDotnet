using System;
using System.Collections.Generic;
using System.Text.Json;
using System.Text.Json.Serialization;
using GaugeDotnet.Gauges.Custom;

namespace GaugeDotnet.Designer;

public class GaugeDesignerViewModel
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
    };

    public CustomGaugeDefinition Definition { get; private set; }
    public GaugeElement? SelectedElement { get; private set; }

    private int _elementCounter;

    public GaugeDesignerViewModel()
    {
        Definition = CreateDefaultGauge();
        _elementCounter = Definition.Elements.Count;
    }

    public void New()
    {
        Definition = new CustomGaugeDefinition();
        SelectedElement = null;
        _elementCounter = 0;
    }

    public void Load(CustomGaugeDefinition definition)
    {
        Definition = definition;
        SelectedElement = null;
        _elementCounter = definition.Elements.Count;
    }

    public GaugeElement AddElement(GaugeElement element)
    {
        _elementCounter++;
        if (string.IsNullOrEmpty(element.Name))
            element.Name = $"{element.TypeLabel} {_elementCounter}";
        Definition.Elements.Add(element);
        SelectedElement = element;
        return element;
    }

    public void DeleteSelected()
    {
        if (SelectedElement == null) return;
        Definition.Elements.Remove(SelectedElement);
        SelectedElement = null;
    }

    public GaugeElement? Duplicate()
    {
        if (SelectedElement == null) return null;
        string json = JsonSerializer.Serialize<GaugeElement>(SelectedElement, JsonOptions);
        GaugeElement? copy = JsonSerializer.Deserialize<GaugeElement>(json, JsonOptions);
        if (copy == null) return null;

        _elementCounter++;
        copy.Id = Guid.NewGuid().ToString("N")[..8];
        copy.Name = $"{copy.Name} copy";
        copy.X += 20;
        copy.Y += 20;
        Definition.Elements.Add(copy);
        SelectedElement = copy;
        return copy;
    }

    public void MoveElement(int direction)
    {
        if (SelectedElement == null) return;
        int idx = Definition.Elements.IndexOf(SelectedElement);
        int newIdx = idx + direction;
        if (newIdx < 0 || newIdx >= Definition.Elements.Count) return;
        Definition.Elements.RemoveAt(idx);
        Definition.Elements.Insert(newIdx, SelectedElement);
    }

    public void SelectElement(GaugeElement? element)
    {
        SelectedElement = element;
    }

    public static CustomGaugeDefinition CreateDefaultGauge() => new()
    {
        Elements =
        [
            new ArcElement
            {
                Name = "RPM Arc",
                X = 320, Y = 240,
                Radius = 200, StrokeWidth = 30,
                StartAngleDeg = 135, SweepAngleDeg = 270,
                Color = "#00FFFF", TrackColor = "#0A1A1A",
                ShowTrack = true, IsDynamic = true,
                DataSource = "Rpm", MinValue = 0, MaxValue = 8000,
            },
            new TickRingElement
            {
                Name = "RPM Ticks",
                X = 320, Y = 240,
                Radius = 178, StartAngleDeg = 135, SweepAngleDeg = 270,
                MajorCount = 8, MinorPerMajor = 4,
                MajorLength = 15, MinorLength = 8,
                MajorWidth = 2, MinorWidth = 1,
                Color = "#AAAAAA",
                ShowLabels = true, LabelFontSize = 14,
                LabelColor = "#888888", LabelOffset = 22,
                MinValue = 0, MaxValue = 8000,
            },
            new NeedleElement
            {
                Name = "RPM Needle",
                X = 320, Y = 240,
                Length = 170, TailLength = 25, NeedleWidth = 4,
                StartAngleDeg = 135, SweepAngleDeg = 270,
                Color = "#FF3333",
                ShowHub = true, HubRadius = 10, HubColor = "#CCCCCC",
                DataSource = "Rpm", MinValue = 0, MaxValue = 8000,
            },
            new ValueDisplayElement
            {
                Name = "RPM Value",
                X = 320, Y = 340,
                FontSize = 42, Color = "#00FFFF",
                Font = "DSEG7 Classic", Format = "F0",
                DataSource = "Rpm", MinValue = 0, MaxValue = 8000,
            },
            new TextElement
            {
                Name = "Title",
                X = 320, Y = 390,
                Text = "RPM", FontSize = 20,
                Color = "#666666", Font = "Race Sport",
            },
            new CircleElement
            {
                Name = "Hub Cap",
                X = 320, Y = 240,
                Radius = 8, FillColor = "#444444",
                StrokeColor = "#666666", CircleStrokeWidth = 1,
            },
        ]
    };
}
