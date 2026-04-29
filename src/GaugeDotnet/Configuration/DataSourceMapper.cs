using ME1_4NET;

namespace GaugeDotnet.Configuration
{
    public static class DataSourceMapper
    {
        private static readonly Dictionary<string, Func<MEData, decimal>> Mappings = new(StringComparer.OrdinalIgnoreCase)
        {
            ["Rpm"] = d => d.Rpm,
            ["ThrottlePosition"] = d => (decimal)d.ThrottlePosition,
            ["Map"] = d => (decimal)d.Map,
            ["Iat"] = d => (decimal)d.Iat,
            ["RpmHardLimit"] = d => d.RpmHardLimit,
            ["AfrCurr1"] = d => d.AfrCurr1,
            ["AfrCurr2"] = d => d.AfrCurr2,
            ["LambdaTrim"] = d => d.LambdaTrim,
            ["AfrTarget"] = d => d.AfrTarget,
            ["FuelEthPerc"] = d => d.FuelEthPerc,
            ["IgnAdvAngle"] = d => d.IgnAdvAngle,
            ["IgnDwell"] = d => d.IgnDwell,
            ["PriInjAngle"] = d => d.PriInjAngle,
            ["PriInjPw"] = d => d.PriInjPw,
            ["OilTemp"] = d => d.OilTemp,
            ["OilPressure"] = d => d.OilPressure,
            ["FuelPressure"] = d => d.FuelPressure,
            ["BatteryVoltage"] = d => d.BatteryVoltage,
            ["KnockLevel"] = d => d.KnockLevel,
            ["InjectorDuty"] = d => d.InjectorDuty,
            ["GearPos"] = d => d.GearPos,
            ["MapTarget"] = d => d.MapTarget,
            ["VehicleSpeed"] = d => d.VehicleSpeed,
            ["Egt"] = d => d.Egt,
            ["TurboSpeed"] = d => d.TurboSpeed,
            ["WastegateDuty"] = d => d.WastegateDuty,
            ["TpsVoltage"] = d => d.TpsVoltage,
            ["CamAngle"] = d => d.CamAngle,
            ["Baro"] = d => d.Baro,
        };

        public static List<string> DataSourceNames { get; } = new(Mappings.Keys);

        public static decimal ReadValue(MEData data, string dataSource)
        {
            if (Mappings.TryGetValue(dataSource, out Func<MEData, decimal>? getter))
            {
                return getter(data);
            }

            throw new ArgumentException($"Unknown data source: '{dataSource}'. Valid sources: {string.Join(", ", Mappings.Keys)}");
        }
    }
}
