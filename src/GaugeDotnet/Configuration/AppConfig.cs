namespace GaugeDotnet.Configuration
{
    public class AppConfig
    {
        public const int MaxScreens = 10;

        public bool DemoMode { get; set; } = false;

        /// <summary>
        /// Override system battery with a fixed percentage (0-100). Useful for PC testing.
        /// Set to e.g. 20 to trigger the low-battery overlay without real hardware.
        /// Leave null to read from /sys/class/power_supply.
        /// </summary>
        public int? SimulateBatteryPercent { get; set; } = null;

        public string? DeviceMacAddress { get; set; } = "20:E7:C8:B9:20:2A";

        public List<ScreenConfig> Screens { get; set; } = new();
    }
}
