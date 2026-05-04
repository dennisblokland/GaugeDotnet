namespace ME1_4NET.Frames
{
    /// <summary>
    /// Frame5 (PID 0x304):
    /// - oil_temp: bytes 0-1, i16 * 0.1, deg C
    /// - oil_pressure: bytes 2-3, i16 * 0.1, kPa
    /// - clt: bytes 4-5, i16 * 0.1, deg C
    /// - vbat: bytes 6-7, i16 * 0.1, V
    /// </summary>
    public struct ME1_5 : ICanFrame
    {
        public float OilTemp { get; }
        public float OilPressure { get; }
        public float Clt { get; }
        public float Vbat { get; }

        private ME1_5(float oilTemp, float oilPressure, float clt, float vbat)
        {
            OilTemp = oilTemp;
            OilPressure = oilPressure;
            Clt = clt;
            Vbat = vbat;
        }

        public static ME1_5 Decode(byte[] payload)
        {
            if (payload.Length < 8)
                throw new ArgumentException("Payload too short for Frame5");

            short oilTempRaw = (short)((payload[1] << 8) | payload[0]);
            float oilTemp = oilTempRaw * 0.1f;
            short oilPressRaw = (short)((payload[3] << 8) | payload[2]);
            float oilPressure = oilPressRaw * 0.1f;
            short cltRaw = (short)((payload[5] << 8) | payload[4]);
            float clt = cltRaw * 0.1f;
            short vbatRaw = (short)((payload[7] << 8) | payload[6]);
            float vbat = vbatRaw * 0.1f;

            return new ME1_5(oilTemp, oilPressure, clt, vbat);
        }
    }
}