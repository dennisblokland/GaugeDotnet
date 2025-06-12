namespace ME1_4NET.Frames
{
    /// <summary>
    /// Frame1 (PID 0x10):
    /// - rpm: bytes 0-1, u16
    /// - throttle_position: bytes 2-3, i16 * 0.1f
    /// - map: bytes 4-5, u16 * 0.01f
    /// - iat: bytes 6-7, i16 * 0.1f
    /// </summary>
    public struct ME1_1 : ICanFrame
    {
        public ushort Rpm { get; }
        public float ThrottlePosition { get; }
        public float Map { get; }
        public float Iat { get; }

        private ME1_1(ushort rpm, float throttlePosition, float map, float iat)
        {
            Rpm = rpm;
            ThrottlePosition = throttlePosition;
            Map = map;
            Iat = iat;
        }

        public static ME1_1 Decode(byte[] payload)
        {
            if (payload.Length < 8)
                throw new ArgumentException("Payload too short for Frame1");

            ushort rpm = (ushort)((payload[1] << 8) | payload[0]);
            short thrRaw = (short)((payload[3] << 8) | payload[2]);
            float throttle = thrRaw * 0.1f;
            ushort mapRaw = (ushort)((payload[5] << 8) | payload[4]);
            float map = mapRaw * 0.01f;
            short iatRaw = (short)((payload[7] << 8) | payload[6]);
            float iat = iatRaw * 0.1f;

            return new ME1_1(rpm, throttle, map, iat);
        }
    }
}