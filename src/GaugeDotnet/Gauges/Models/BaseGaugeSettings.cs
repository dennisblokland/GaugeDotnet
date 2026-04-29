namespace GaugeDotnet.Gauges.Models
{
    public class BaseGaugeSettings
    {
        public float InitialValue { get; set; } = 0;
        public float MinValue { get; set; } = 0;
        public float MaxValue { get; set; } = 100;
        public string Unit { get; set; } = "";
        public string Title { get; set; } = "";
    }
}
