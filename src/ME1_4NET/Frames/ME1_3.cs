namespace ME1_4NET.Frames
{
    /// <summary>
    /// Frame3 (PID 0x12):
    /// - ign_adv_angle: bytes 0-1, i16
    /// - ign_dwell: bytes 2-3, i16
    /// - pri_inj_angle: bytes 4-5, i16
    /// - pri_inj_pw: bytes 6-7, i16
    /// </summary>
    public struct ME1_3 : ICanFrame
    {
        public short IgnAdvAngle { get; }
        public short IgnDwell { get; }
        public short PriInjAngle { get; }
        public short PriInjPw { get; }

        private ME1_3(short ignAdv, short ignDwell, short priAngle, short priPw)
        {
            IgnAdvAngle = ignAdv;
            IgnDwell = ignDwell;
            PriInjAngle = priAngle;
            PriInjPw = priPw;
        }

        public static ME1_3 Decode(byte[] payload)
        {
            if (payload.Length < 8)
                throw new ArgumentException("Payload too short for Frame3");

            short ignAdv = (short)((payload[1] << 8) | payload[0]);
            short dwell = (short)((payload[3] << 8) | payload[2]);
            short priAngle = (short)((payload[5] << 8) | payload[4]);
            short priPw = (short)((payload[7] << 8) | payload[6]);

            return new ME1_3(ignAdv, dwell, priAngle, priPw);
        }
    }
}