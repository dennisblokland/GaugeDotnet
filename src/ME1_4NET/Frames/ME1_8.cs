namespace ME1_4NET.Frames
{
    /// <summary>
    /// Frame8 (PID 0x307):
    /// - egt_1: bytes 0-1, u16 * 0.1, deg C
    /// - egt_2: bytes 2-3, u16 * 0.1, deg C
    /// - gpt_1: bytes 4-5, i16
    /// - gpt_2: bytes 6-7, i16
    /// </summary>
    public struct ME1_8 : ICanFrame
    {
        public float Egt1 { get; }
        public float Egt2 { get; }
        public short Gpt1 { get; }
        public short Gpt2 { get; }

        private ME1_8(float egt1, float egt2, short gpt1, short gpt2)
        {
            Egt1 = egt1;
            Egt2 = egt2;
            Gpt1 = gpt1;
            Gpt2 = gpt2;
        }

        public static ME1_8 Decode(ReadOnlySpan<byte> payload)
        {
            if (payload.Length < 8)
                throw new ArgumentException("Payload too short for Frame8");

            ushort egt1Raw = (ushort)((payload[1] << 8) | payload[0]);
            float egt1 = egt1Raw * 0.1f;
            ushort egt2Raw = (ushort)((payload[3] << 8) | payload[2]);
            float egt2 = egt2Raw * 0.1f;
            short gpt1 = (short)((payload[5] << 8) | payload[4]);
            short gpt2 = (short)((payload[7] << 8) | payload[6]);

            return new ME1_8(egt1, egt2, gpt1, gpt2);
        }
    }
}