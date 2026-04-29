namespace GaugeDotnet.Gauges.Models
{
	public class CircularGaugeSettings : BaseGaugeSettings
	{
		public int SegmentCount { get; set; } = 64;
		public bool Smoothing { get; set; } = true;
		public int Decimals { get; set; } = 0;
	}
}
