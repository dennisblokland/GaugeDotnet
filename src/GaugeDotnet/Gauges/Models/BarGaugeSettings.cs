namespace GaugeDotnet.Gauges.Models
{
    public class BarGaugeSettings : BaseGaugeSettings
    {
        public int SegmentCount { get; set; } = 32;
        public bool Smoothing { get; set; } = true;
        public int Decimals { get; set; } = 0;
  
    }
}
