namespace GaugeDotnet.Gauges.Models
{
    public class NeedleGaugeSettings : BaseGaugeSettings
    {
        public bool Smoothing { get; set; } = true;
        public int Decimals { get; set; } = 0;
    }
}
