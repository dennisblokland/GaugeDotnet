using System.Collections.Generic;
using System.ComponentModel.DataAnnotations;

namespace GaugeDotnet.Configuration
{
    public class AppConfig
    {
        public const int MaxScreens = 10;

        [MaxLength(MaxScreens)]
        public List<ScreenConfig> Screens { get; set; } = new();
    }
}
