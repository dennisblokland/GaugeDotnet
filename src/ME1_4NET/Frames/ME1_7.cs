namespace ME1_4NET.Frames
{
    /// <summary>
    /// Frame7 (PID 0x306):
    /// - knock_peak_reading: bytes 0-1, u16
    /// - knock_ign_adv_mod: byte 2, i8 * 0.1, deg
    /// - fuel_pressure: byte 3, u8 * 5, kPa
    /// - fuel_temp: byte 4, u8, deg C
    /// - knock_evs_cnt: bytes 5-6, u16
    /// </summary>
    public struct ME1_7 : ICanFrame
    {
        public ushort KnockPeakReading { get; }
        public float KnockIgnAdvMod { get; }
        public ushort FuelPressure { get; }
        public byte FuelTemp { get; }
        public ushort KnockEvsCnt { get; }

        private ME1_7(ushort knockPeak, float knockIgnAdv, ushort fuelPress, byte fuelTemp, ushort knockEvs)
        {
            KnockPeakReading = knockPeak;
            KnockIgnAdvMod = knockIgnAdv;
            FuelPressure = fuelPress;
            FuelTemp = fuelTemp;
            KnockEvsCnt = knockEvs;
        }

        public static ME1_7 Decode(ReadOnlySpan<byte> payload)
        {
            if (payload.Length < 7)
                throw new ArgumentException("Payload too short for Frame7");

            ushort knockPeak = (ushort)((payload[1] << 8) | payload[0]);
            sbyte knockIgnRaw = unchecked((sbyte)payload[2]);
            float knockIgnAdvMod = knockIgnRaw * 0.1f;
            ushort fuelPress = (ushort)(payload[3] * 5);
            byte fuelTemp = payload[4];
            ushort knockEvs = (ushort)((payload[6] << 8) | payload[5]);

            return new ME1_7(knockPeak, knockIgnAdvMod, fuelPress, fuelTemp, knockEvs);
        }
    }
}