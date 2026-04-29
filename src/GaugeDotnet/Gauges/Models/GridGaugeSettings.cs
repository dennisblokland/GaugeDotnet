using GaugeDotnet.Configuration;

namespace GaugeDotnet.Gauges.Models
{
	public class GridGaugeSettings : BaseGaugeSettings
	{
		public List<GridCellConfig> Cells { get; set; } = new();
	}
}
