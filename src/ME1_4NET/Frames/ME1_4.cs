namespace ME1_4NET.Frames
{
    /// <summary>
    /// Frame4 (PID 0x13):
    /// - oil_temp: byte 0 as i8
    /// - oil_pressure: bytes 1-2, u16
    /// - fuel_pressure: bytes 3-4, u16
    /// </summary>
    public struct ME1_4 : ICanFrame
    {
        public sbyte OilTemp { get; }
        public ushort OilPressure { get; }
        public ushort FuelPressure { get; }

        private ME1_4(sbyte oilTemp, ushort oilPress, ushort fuelPress)
        {
            OilTemp = oilTemp;
            OilPressure = oilPress;
            FuelPressure = fuelPress;
        }

        public static ME1_4 Decode(byte[] payload)
        {
            if (payload.Length < 5)
                throw new ArgumentException("Payload too short for Frame4");

            sbyte oilTemp = unchecked((sbyte)payload[0]);
            ushort oilPress = (ushort)((payload[2] << 8) | payload[1]);
            ushort fuelPress = (ushort)((payload[4] << 8) | payload[3]);

            return new ME1_4(oilTemp, oilPress, fuelPress);
        }
    }
}