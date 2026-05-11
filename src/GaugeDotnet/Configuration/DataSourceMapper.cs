using System.Diagnostics.CodeAnalysis;
using System.Linq.Expressions;
using System.Reflection;
using ME1_4NET;

namespace GaugeDotnet.Configuration
{
    public static class DataSourceMapper
    {
        private static readonly Dictionary<string, Func<MEData, float>> Mappings;

        public static IReadOnlyList<string> DataSourceNames { get; }

        [UnconditionalSuppressMessage("Trimming", "IL2026")]
        [UnconditionalSuppressMessage("Trimming", "IL2075")]
        static DataSourceMapper()
        {
            var mappings = new Dictionary<string, Func<MEData, float>>(StringComparer.OrdinalIgnoreCase);

            ParameterExpression param = Expression.Parameter(typeof(MEData), "d");
            foreach (PropertyInfo prop in typeof(MEData).GetProperties(BindingFlags.Public | BindingFlags.Instance))
            {
                if (!IsNumericType(prop.PropertyType)) continue;
                UnaryExpression body = Expression.Convert(Expression.Property(param, prop), typeof(float));
                mappings[prop.Name] = Expression.Lambda<Func<MEData, float>>(body, param).Compile();
            }

            // Backwards-compat aliases kept for existing gauge configs
            mappings["BatteryVoltage"] = d => d.Vbat;
            mappings["KnockLevel"] = d => d.KnockPeakReading;
            mappings["InjectorDuty"] = d => d.PriInjDuty;

            Mappings = mappings;
            DataSourceNames = new List<string>(mappings.Keys).AsReadOnly();
        }

        private static bool IsNumericType(Type t) =>
            t == typeof(float)  || t == typeof(double) ||
            t == typeof(int)    || t == typeof(uint)   ||
            t == typeof(short)  || t == typeof(ushort) ||
            t == typeof(long)   || t == typeof(ulong)  ||
            t == typeof(byte)   || t == typeof(sbyte);

        public static float ReadValue(MEData data, string dataSource)
        {
            if (Mappings.TryGetValue(dataSource, out Func<MEData, float>? getter))
                return getter(data);

            throw new ArgumentException($"Unknown data source: '{dataSource}'. Valid sources: {string.Join(", ", Mappings.Keys)}");
        }
    }
}
