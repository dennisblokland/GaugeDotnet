using ME1_4NET.Frames;

namespace ME1_4NET
{
    public class MEData
    {
        // ME1_1
        public ushort Rpm { get; set; }
        public float ThrottlePosition { get; set; }
        public float Map { get; set; }
        public float Iat { get; set; }

        // ME1_2
        public int RpmHardLimit { get; set; }
        public float AfrCurr1 { get; set; }
        public float AfrCurr2 { get; set; }
        public int LambdaTrim { get; set; }
        public float AfrTarget { get; set; }
        public float FuelEthPerc { get; set; }

        // ME1_3
        public short IgnAdvAngle { get; set; }
        public short IgnDwell { get; set; }
        public short PriInjAngle { get; set; }
        public short PriInjPw { get; set; }

        // ME1_4
        public float PriInjDuty { get; set; }
        public float SecInjDuty { get; set; }
        public float SecInjAngle { get; set; }
        public float SecInjPw { get; set; }
        public float BoostCtrlDuty { get; set; }

        // ME1_5
        public float OilTemp { get; set; }
        public float OilPressure { get; set; }
        public float Clt { get; set; }
        public float Vbat { get; set; }

        // ME1_6
        public byte GearPos { get; set; }
        public ushort MapTarget { get; set; }
        public ushort VehicleSpeed { get; set; }
        public ushort EpsEvMsk { get; set; }

        // ME1_7
        public ushort KnockPeakReading { get; set; }
        public float KnockIgnAdvMod { get; set; }
        public ushort FuelPressure { get; set; }
        public byte FuelTemp { get; set; }
        public ushort KnockEvsCnt { get; set; }

        // ME1_8
        public float Egt1 { get; set; }
        public float Egt2 { get; set; }
        public short Gpt1 { get; set; }
        public short Gpt2 { get; set; }

        public void Apply(ICanFrame frame)
        {
            switch (frame)
            {
                case ME1_1 f:
                    Rpm = f.Rpm;
                    ThrottlePosition = f.ThrottlePosition;
                    Map = f.Map;
                    Iat = f.Iat;
                    break;
                case ME1_2 f:
                    RpmHardLimit = f.RpmHardLimit;
                    AfrCurr1 = f.AfrCurr1;
                    AfrCurr2 = f.AfrCurr2;
                    LambdaTrim = f.LambdaTrim;
                    AfrTarget = f.AfrTarget;
                    FuelEthPerc = f.FuelEthPerc;
                    break;
                case ME1_3 f:
                    IgnAdvAngle = f.IgnAdvAngle;
                    IgnDwell = f.IgnDwell;
                    PriInjAngle = f.PriInjAngle;
                    PriInjPw = f.PriInjPw;
                    break;
                case ME1_4 f:
                    PriInjDuty = f.PriInjDuty;
                    SecInjDuty = f.SecInjDuty;
                    SecInjAngle = f.SecInjAngle;
                    SecInjPw = f.SecInjPw;
                    BoostCtrlDuty = f.BoostCtrlDuty;
                    break;
                case ME1_5 f:
                    OilTemp = f.OilTemp;
                    OilPressure = f.OilPressure;
                    Clt = f.Clt;
                    Vbat = f.Vbat;
                    break;
                case ME1_6 f:
                    GearPos = f.GearPos;
                    MapTarget = f.MapTarget;
                    VehicleSpeed = f.VehicleSpeed;
                    EpsEvMsk = f.EpsEvMsk;
                    break;
                case ME1_7 f:
                    KnockPeakReading = f.KnockPeakReading;
                    KnockIgnAdvMod = f.KnockIgnAdvMod;
                    FuelPressure = f.FuelPressure;
                    FuelTemp = f.FuelTemp;
                    KnockEvsCnt = f.KnockEvsCnt;
                    break;
                case ME1_8 f:
                    Egt1 = f.Egt1;
                    Egt2 = f.Egt2;
                    Gpt1 = f.Gpt1;
                    Gpt2 = f.Gpt2;
                    break;
            }
        }
    }
}
