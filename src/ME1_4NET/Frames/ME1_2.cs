namespace ME1_4Net.Frames
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
        public decimal AfrCurr1 { get; }
        public decimal AfrCurr2 { get; }
        public int LambdaTrim { get; }
        public decimal AfrTarget { get; }
        public decimal FuelEthPerc { get; }

        private ME1_2(int rpmHardLimit, decimal afr1, decimal afr2, int lambdaTrim, decimal afrTarget, decimal fuelEthPerc)
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
            decimal afr1 = (decimal)(payload[2] * 0.05 + 7.5);
            decimal afr2 = (decimal)(payload[3] * 0.05 + 7.5);
            int lambda = (payload[5] << 8) | payload[4];
            decimal afrTarget = payload[6];
            decimal fuelEth = payload[7];

            return new ME1_2(rpmHard, afr1, afr2, lambda, afrTarget, fuelEth);
        }
    }
}