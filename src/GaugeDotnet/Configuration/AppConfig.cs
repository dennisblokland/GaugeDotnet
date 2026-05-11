using System.ComponentModel.DataAnnotations;
using System.Diagnostics.CodeAnalysis;

namespace GaugeDotnet.Configuration
{
    public class AppConfig
    {
        public const int MaxScreens = 10;

        public bool DemoMode { get; set; } = false;

        public string? DeviceMacAddress { get; set; } = "20:E7:C8:B9:20:2A";

        public List<ScreenConfig> Screens { get; set; } = new();
    }
}
