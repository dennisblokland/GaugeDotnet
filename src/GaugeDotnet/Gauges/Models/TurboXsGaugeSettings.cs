namespace GaugeDotnet.Gauges.Models
{
    public class TurboXsGaugeSettings : BaseGaugeSettings
    {
        public bool Smoothing { get; set; } = true;
        public int Decimals { get; set; } = 1;
        public string StatusMode { get; set; } = "Peak";
        public string StatusLabel { get; set; } = "";
        public string StatusText { get; set; } = "Stop";
        public bool Lcd { get; set; } = false;
        public float LcdGhosting { get; set; } = 0.3f;
    }
}
