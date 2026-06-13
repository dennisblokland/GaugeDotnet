using System.Text.Json.Serialization;

namespace GaugeDotnet.Configuration
{
    [JsonConverter(typeof(TolerantGaugeTypeConverter))]
    public enum GaugeType
    {
        Bar,
        Circular,
        Histogram,
        Needle,
        Digital,
        Sweep,
        MinMax,
        Grid,
        Custom
    }
}
