namespace GaugeDotnet.Configuration
{
    public class ScreenConfig
    {
        public GaugeConfig Gauge { get; set; } = new();
        public string? CustomDefinitionFile { get; set; }
    }
}
