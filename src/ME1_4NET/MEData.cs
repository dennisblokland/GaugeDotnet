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
        public decimal AfrCurr1 { get; set; }
        public decimal AfrCurr2 { get; set; }
        public int LambdaTrim { get; set; }
        public decimal AfrTarget { get; set; }
        public decimal FuelEthPerc { get; set; }

        // ME1_3
        public short IgnAdvAngle { get; set; }
        public short IgnDwell { get; set; }
        public short PriInjAngle { get; set; }
        public short PriInjPw { get; set; }

        // ME1_4
        public sbyte OilTemp { get; set; }
        public ushort OilPressure { get; set; }
        public ushort FuelPressure { get; set; }

        // ME1_5
        public ushort BatteryVoltage { get; set; }
        public byte KnockLevel { get; set; }
        public byte InjectorDuty { get; set; }

        // ME1_6
        public byte GearPos { get; set; }
        public ushort MapTarget { get; set; }
        public ushort VehicleSpeed { get; set; }
        public ushort EpsEvMsk { get; set; }

        // ME1_7
        public ushort Egt { get; set; }
        public ushort TurboSpeed { get; set; }
        public byte WastegateDuty { get; set; }

        // ME1_8
        public ushort TpsVoltage { get; set; }
        public short CamAngle { get; set; }
        public ushort Baro { get; set; }

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
                    OilTemp = f.OilTemp;
                    OilPressure = f.OilPressure;
                    FuelPressure = f.FuelPressure;
                    break;
                case ME1_5 f:
                    BatteryVoltage = f.BatteryVoltage;
                    KnockLevel = f.KnockLevel;
                    InjectorDuty = f.InjectorDuty;
                    break;
                case ME1_6 f:
                    GearPos = f.GearPos;
                    MapTarget = f.MapTarget;
                    VehicleSpeed = f.VehicleSpeed;
                    EpsEvMsk = f.EpsEvMsk;
                    break;
                case ME1_7 f:
                    Egt = f.Egt;
                    TurboSpeed = f.TurboSpeed;
                    WastegateDuty = f.WastegateDuty;
                    break;
                case ME1_8 f:
                    TpsVoltage = f.TpsVoltage;
                    CamAngle = f.CamAngle;
                    Baro = f.Baro;
                    break;
            }
        }
    }
}
