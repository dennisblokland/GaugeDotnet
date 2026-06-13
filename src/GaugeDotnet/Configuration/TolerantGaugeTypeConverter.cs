using System.Text.Json;
using System.Text.Json.Serialization;

namespace GaugeDotnet.Configuration
{
    /// <summary>
    /// Reads <see cref="GaugeType"/> as a string but falls back to a safe default
    /// for unknown/legacy values instead of throwing, so one bad screen in the
    /// config cannot crash startup or wipe the whole file.
    /// </summary>
    public sealed class TolerantGaugeTypeConverter : JsonConverter<GaugeType>
    {
        private const GaugeType Fallback = GaugeType.Bar;

        public override GaugeType Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
        {
            if (reader.TokenType == JsonTokenType.String)
            {
                string? s = reader.GetString();
                if (Enum.TryParse(s, ignoreCase: true, out GaugeType result))
                    return result;
            }
            else if (reader.TokenType == JsonTokenType.Number && reader.TryGetInt32(out int n)
                && Enum.IsDefined(typeof(GaugeType), n))
            {
                return (GaugeType)n;
            }

            return Fallback;
        }

        public override void Write(Utf8JsonWriter writer, GaugeType value, JsonSerializerOptions options)
        {
            writer.WriteStringValue(value.ToString());
        }
    }
}
