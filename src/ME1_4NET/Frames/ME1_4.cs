namespace ME1_4NET.Frames
{
    /// <summary>
    /// Frame4 (PID 0x303):
    /// - pri_inj_duty: byte 0, u8 * 0.5, %
    /// - sec_inj_duty: byte 1, u8 * 0.5, %
    /// - sec_inj_angle: bytes 2-3, i16 * 0.1, deg
    /// - sec_inj_pw: bytes 4-5, i16 * 0.1, ms
    /// - boost_ctrl_duty: byte 6, u8 * 0.5, %
    /// </summary>
    public struct ME1_4 : ICanFrame
    {
        public float PriInjDuty { get; }
        public float SecInjDuty { get; }
        public float SecInjAngle { get; }
        public float SecInjPw { get; }
        public float BoostCtrlDuty { get; }

        private ME1_4(float priInjDuty, float secInjDuty, float secInjAngle, float secInjPw, float boostCtrlDuty)
        {
            PriInjDuty = priInjDuty;
            SecInjDuty = secInjDuty;
            SecInjAngle = secInjAngle;
            SecInjPw = secInjPw;
            BoostCtrlDuty = boostCtrlDuty;
        }

        public static ME1_4 Decode(ReadOnlySpan<byte> payload)
        {
            if (payload.Length < 7)
                throw new ArgumentException("Payload too short for Frame4");

            float priInjDuty = payload[0] * 0.5f;
            float secInjDuty = payload[1] * 0.5f;
            short secInjAngleRaw = (short)((payload[3] << 8) | payload[2]);
            float secInjAngle = secInjAngleRaw * 0.1f;
            short secInjPwRaw = (short)((payload[5] << 8) | payload[4]);
            float secInjPw = secInjPwRaw * 0.1f;
            float boostCtrlDuty = payload[6] * 0.5f;

            return new ME1_4(priInjDuty, secInjDuty, secInjAngle, secInjPw, boostCtrlDuty);
        }
    }
}