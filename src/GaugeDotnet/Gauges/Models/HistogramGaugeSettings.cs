namespace GaugeDotnet.Gauges.Models
{
	public class HistogramGaugeSettings : BaseGaugeSettings
	{
		public int MaxDataPoints { get; set; } = 50;
		public int IntervalMs { get; set; } = 1000;
		public int Decimals { get; set; } = 0;
	}
}
