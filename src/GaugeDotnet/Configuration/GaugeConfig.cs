namespace GaugeDotnet.Configuration
{
    public class GaugeConfig
    {
        public GaugeType Type { get; set; } = GaugeType.Bar;
        public string DataSource { get; set; } = "AfrCurr1";
        public string ColorHex { get; set; } = "#00FFFF";
        public string Title { get; set; } = "";
        public string Unit { get; set; } = "";
        public float MinValue { get; set; } = 0;
        public float MaxValue { get; set; } = 100;
        public float InitialValue { get; set; } = 0;

        // Bar gauge specific
        public int SegmentCount { get; set; } = 32;
        public bool Smoothing { get; set; } = true;
        public int Decimals { get; set; } = 0;

        // Histogram gauge specific
        public int MaxDataPoints { get; set; } = 50;
        public int IntervalMs { get; set; } = 1000;

        // TurboXs gauge specific: the status block under the title.
        // StatusMode is Peak, Min, Text or None; StatusLabel overrides the first line
        // ("Peak:", "Min:" or "Datalog:"), and StatusText is the second line in Text mode.
        public string StatusMode { get; set; } = "Peak";
        public string StatusLabel { get; set; } = "";
        public string StatusText { get; set; } = "Stop";
        // Old-LCD look: low resolution, pixel grid, blue backlight tint and ghosting.
        public bool Lcd { get; set; } = false;
        // Share of the previous frame that lingers in Lcd mode (0 = no ghosting, up to 0.9).
        public float LcdGhosting { get; set; } = 0.2f;

        // Grid gauge specific
        public List<GridCellConfig> Cells { get; set; } = new();
    }
}
