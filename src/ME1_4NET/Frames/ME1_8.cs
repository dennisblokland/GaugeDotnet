namespace ME1_4Net.Frames
{
    /// <summary>
    /// Frame8 (PID 0x17):
    /// - tps_voltage: bytes 0-1, u16
    /// - cam_angle: bytes 2-3, i16
    /// - baro: bytes 4-5, u16
    /// </summary>
    public struct ME1_8 : ICanFrame
    {
        public ushort TpsVoltage { get; }
        public short CamAngle { get; }
        public ushort Baro { get; }

        private ME1_8(ushort tpsVolts, short camAngle, ushort baro)
        {
            TpsVoltage = tpsVolts;
            CamAngle = camAngle;
            Baro = baro;
        }

        public static ME1_8 Decode(byte[] payload)
        {
            if (payload.Length < 6)
                throw new ArgumentException("Payload too short for Frame8");

            ushort tps = (ushort)((payload[1] << 8) | payload[0]);
            short cam = (short)((payload[3] << 8) | payload[2]);
            ushort baro = (ushort)((payload[5] << 8) | payload[4]);

            return new ME1_8(tps, cam, baro);
        }
    }
}