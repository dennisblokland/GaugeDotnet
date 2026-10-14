using System.Text.Json;
using System.Text.Json.Serialization;

namespace GaugeDotnet.Configuration
{
    [JsonSourceGenerationOptions(WriteIndented = true, UseStringEnumConverter = true)]
    [JsonSerializable(typeof(AppConfig))]
    internal partial class AppConfigJsonContext : JsonSerializerContext
    {
    }

    public static class ConfigService
    {
        private static readonly JsonSerializerOptions JsonOptions = AppConfigJsonContext.Default.Options;

        public static string DefaultPath =>
            Path.Combine(AppContext.BaseDirectory, "gauges.json");

        public static AppConfig Load(string? path = null)
        {
            string filePath = path ?? DefaultPath;

            AppConfig? config = null;
            try
            {
                if (File.Exists(filePath))
                {
                    string json = File.ReadAllText(filePath);
                    config = JsonSerializer.Deserialize(json, AppConfigJsonContext.Default.AppConfig);
                }
            }
            catch (JsonException e)
            {
                // Empty or truncated (say, by a power cut): keep it for recovery
                // as gauges.json.bad before the defaults replace it.
                Console.WriteLine($"[config] {filePath}: {e.Message}; using defaults");
                BackupBad(filePath);
            }

            if (config is null || config.Screens.Count == 0)
            {
                config = CreateDefault();
                try
                {
                    Save(config, filePath);
                }
                catch (Exception e) when (e is IOException or UnauthorizedAccessException)
                {
                    Console.WriteLine($"[config] could not write {filePath}: {e.Message}");
                }
            }

            // Enforce max screens
            if (config.Screens.Count > AppConfig.MaxScreens)
            {
                config.Screens.RemoveRange(AppConfig.MaxScreens, config.Screens.Count - AppConfig.MaxScreens);
            }

            return config;
        }

        // Writes atomically: the JSON goes to gauges.json.tmp, is fsynced, then
        // renamed over gauges.json. A power cut on the SD card leaves either the
        // old or the new file, never a truncated one.
        public static void Save(AppConfig config, string? path = null)
        {
            string filePath = path ?? DefaultPath;
            string tmpPath = filePath + ".tmp";
            string json = JsonSerializer.Serialize(config, AppConfigJsonContext.Default.AppConfig);
            try
            {
                using (FileStream stream = new(tmpPath, FileMode.Create, FileAccess.Write, FileShare.None))
                using (StreamWriter writer = new(stream))
                {
                    writer.Write(json);
                    writer.Flush();
                    stream.Flush(flushToDisk: true);
                }
                File.Move(tmpPath, filePath, overwrite: true);
            }
            catch
            {
                try { File.Delete(tmpPath); } catch { }
                throw;
            }
        }

        private static void BackupBad(string filePath)
        {
            string badPath = filePath + ".bad";
            try
            {
                File.Copy(filePath, badPath, overwrite: true);
                Console.WriteLine($"[config] kept the unreadable config as {badPath}");
            }
            catch (Exception e) when (e is IOException or UnauthorizedAccessException)
            {
                Console.WriteLine($"[config] could not back up {filePath}: {e.Message}");
            }
        }

        private static AppConfig CreateDefault()
        {
            return new AppConfig
            {
                DeviceMacAddress = "20:E7:C8:B9:20:2A",
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
                            InitialValue = 14.7f,
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
