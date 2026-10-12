namespace GaugeDotnet.Gauges.Models
{
    public class DialGaugeSettings : BaseGaugeSettings
    {
        public bool Smoothing { get; set; } = true;
        public int Decimals { get; set; } = 1;
    }
}
