
public struct Me1_4Me1_4
{
    public sbyte OilTemp { get; }
    public ushort OilPressure { get; }
    public ushort FuelPressure { get; }

    public Me1_4Me1_4(sbyte oilTemp, ushort oilPressure, ushort fuelPressure)
    {
        OilTemp = oilTemp;
        OilPressure = oilPressure;
        FuelPressure = fuelPressure;
    }
}

public static class Me1_4Me1_4Parser
{
    /// <summary>
    /// Unpacks a 5-byte array into a Me1_4Me1_4 struct.
    /// Returns null if the buffer is too short.
    /// </summary>
    public static Me1_4Me1_4? Unpack(ReadOnlySpan<byte> data)
    {
        if (data.Length < 5)
            return null;

        // Rust: d[0] as i8
        sbyte oilTemp = unchecked((sbyte)data[0]);

        // Rust big-endian extract_u16!(d, 1, 2):
        ushort oilPressure = (ushort)((data[1] << 8) | data[2]);
        ushort fuelPressure = (ushort)((data[3] << 8) | data[4]);

        return new Me1_4Me1_4(oilTemp, oilPressure, fuelPressure);
    }
}
