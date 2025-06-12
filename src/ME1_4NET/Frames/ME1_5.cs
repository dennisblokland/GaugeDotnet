namespace ME1_4NET.Frames
{
    /// <summary>
    /// Frame5 (PID 0x14):
    /// - battery_voltage: bytes 0-1, u16
    /// - knock_level: byte 2
    /// - injector_duty: byte 3
    /// </summary>
    public struct ME1_5 : ICanFrame
    {
        public ushort BatteryVoltage { get; }
        public byte KnockLevel { get; }
        public byte InjectorDuty { get; }

        private ME1_5(ushort battVolt, byte knock, byte injDuty)
        {
            BatteryVoltage = battVolt;
            KnockLevel = knock;
            InjectorDuty = injDuty;
        }

        public static ME1_5 Decode(byte[] payload)
        {
            if (payload.Length < 4)
                throw new ArgumentException("Payload too short for Frame5");

            ushort batt = (ushort)((payload[1] << 8) | payload[0]);
            byte knock = payload[2];
            byte inj = payload[3];

            return new ME1_5(batt, knock, inj);
        }
    }
}