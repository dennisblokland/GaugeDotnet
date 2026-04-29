namespace GaugeDotnet.Gauges.Models
{
    public class SweepGaugeSettings : BaseGaugeSettings
    {
        public bool Smoothing { get; set; } = true;
        public int Decimals { get; set; } = 0;
    }
}
