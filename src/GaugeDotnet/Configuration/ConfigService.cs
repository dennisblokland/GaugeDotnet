using System.Text.Json;
using System.Text.Json.Serialization;

namespace GaugeDotnet.Configuration
{
    public static class ConfigService
    {
        private static readonly JsonSerializerOptions JsonOptions = new()
        {
            WriteIndented = true,
            Converters = { new JsonStringEnumConverter() }
        };

        public static string DefaultPath =>
            Path.Combine(AppContext.BaseDirectory, "gauges.json");

        public static AppConfig Load(string? path = null)
        {
            string filePath = path ?? DefaultPath;

            if (!File.Exists(filePath))
            {
                AppConfig defaults = CreateDefault();
                Save(defaults, filePath);
                return defaults;
            }

            string json = File.ReadAllText(filePath);
            AppConfig? config = JsonSerializer.Deserialize<AppConfig>(json, JsonOptions);

            if (config is null || config.Screens.Count == 0)
            {
                config = CreateDefault();
                Save(config, filePath);
            }

            // Enforce max screens
            if (config.Screens.Count > AppConfig.MaxScreens)
            {
                config.Screens.RemoveRange(AppConfig.MaxScreens, config.Screens.Count - AppConfig.MaxScreens);
            }

            return config;
        }

        public static void Save(AppConfig config, string? path = null)
        {
            string filePath = path ?? DefaultPath;
            string json = JsonSerializer.Serialize(config, JsonOptions);
            File.WriteAllText(filePath, json);
        }

        private static AppConfig CreateDefault()
        {
            return new AppConfig
            {
                Screens =
                [
                    new ScreenConfig
                    {
                        Gauge = new GaugeConfig
                        {
                            Type = GaugeType.Bar,
                            DataSource = "AfrCurr1",
                            ColorHex = "#00FFFF",
                            Title = "AFR",
                            Unit = "",
                            MinValue = 8,
                            MaxValue = 18,
                            InitialValue = 14.7M,
                            Decimals = 2,
                            SegmentCount = 32,
                            Smoothing = true
                        }
                    }
                ]
            };
        }
    }
}
