namespace GaugeDotnet.Configuration
{
    public class GaugeConfig
    {
        public GaugeType Type { get; set; } = GaugeType.Bar;
        public string DataSource { get; set; } = "AfrCurr1";
        public string ColorHex { get; set; } = "#00FFFF";
        public string Title { get; set; } = "";
        public string Unit { get; set; } = "";
        public decimal MinValue { get; set; } = 0;
        public decimal MaxValue { get; set; } = 100;
        public decimal InitialValue { get; set; } = 0;

        // Bar gauge specific
        public int SegmentCount { get; set; } = 32;
        public bool Smoothing { get; set; } = true;
        public int Decimals { get; set; } = 0;

        // Histogram gauge specific
        public int MaxDataPoints { get; set; } = 50;
        public int IntervalMs { get; set; } = 1000;

        // Grid gauge specific
        public List<GridCellConfig> Cells { get; set; } = new();
    }
}
