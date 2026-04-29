namespace ME1_4NET.Frames
{
    /// <summary>
    /// Frame2 (PID 0x11):
    /// - rpm_hard_limit: bytes 0-1, u16
    /// - afr_curr_1: byte 2
    /// - afr_curr_2: byte 3
    /// - lambda_trim: bytes 4-5, u16
    /// - afr_target: byte 6
    /// - fuel_eth_perc: byte 7
    /// </summary>
    public struct ME1_2 : ICanFrame
    {
        public int RpmHardLimit { get; }
        public float AfrCurr1 { get; }
        public float AfrCurr2 { get; }
        public int LambdaTrim { get; }
        public float AfrTarget { get; }
        public float FuelEthPerc { get; }

        private ME1_2(int rpmHardLimit, float afr1, float afr2, int lambdaTrim, float afrTarget, float fuelEthPerc)
        {
            RpmHardLimit = rpmHardLimit;
            AfrCurr1 = afr1;
            AfrCurr2 = afr2;
            LambdaTrim = lambdaTrim;
            AfrTarget = afrTarget;
            FuelEthPerc = fuelEthPerc;
        }

        public static ME1_2 Decode(byte[] payload)
        {
            if (payload.Length < 8)
                throw new ArgumentException("Payload too short for Frame2");

            int rpmHard = (payload[1] << 8) | payload[0];
            float afr1 = (float)(payload[2] * 0.05 + 7.5);
            float afr2 = (float)(payload[3] * 0.05 + 7.5);
            int lambda = (payload[5] << 8) | payload[4];
            float afrTarget = payload[6];
            float fuelEth = payload[7];

            return new ME1_2(rpmHard, afr1, afr2, lambda, afrTarget, fuelEth);
        }
    }
}