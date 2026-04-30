using System.ComponentModel.DataAnnotations;

namespace GaugeDotnet.Configuration
{
    public class AppConfig
    {
        public const int MaxScreens = 10;

        public bool DemoMode { get; set; } = false;

        public string? DeviceMacAddress { get; set; } = "A0:DD:6C:B3:42:CE";


        [MaxLength(MaxScreens)]
        public List<ScreenConfig> Screens { get; set; } = new();
    }
}
