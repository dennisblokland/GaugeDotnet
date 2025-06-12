namespace ME1_4Net.Frames
{
    /// <summary>
    /// Frame7 (PID 0x16):
    /// - egt: bytes 0-1, u16
    /// - turbo_speed: bytes 2-3, u16
    /// - wastegate_duty: byte 4
    /// </summary>
    public struct ME1_7 : ICanFrame
    {
        public ushort Egt { get; }
        public ushort TurboSpeed { get; }
        public byte WastegateDuty { get; }

        private ME1_7(ushort egt, ushort turbo, byte wasteDuty)
        {
            Egt = egt;
            TurboSpeed = turbo;
            WastegateDuty = wasteDuty;
        }

        public static ME1_7 Decode(byte[] payload)
        {
            if (payload.Length < 5)
                throw new ArgumentException("Payload too short for Frame7");

            ushort egt = (ushort)((payload[1] << 8) | payload[0]);
            ushort turbo = (ushort)((payload[3] << 8) | payload[2]);
            byte duty = payload[4];

            return new ME1_7(egt, turbo, duty);
        }
    }
}