namespace ME1_4Net.Frames
{
    /// <summary>
    /// Frame6 (PID 0x15):
    /// - gear_pos: byte 0
    /// - map_target: bytes 1-2, u16
    /// - vehicle_speed: bytes 3-4, u16
    /// - eps_ev_msk: bytes 5-6, u16
    /// </summary>
    public struct ME1_6 : ICanFrame
    {
        public byte GearPos { get; }
        public ushort MapTarget { get; }
        public ushort VehicleSpeed { get; }
        public ushort EpsEvMsk { get; }

        private ME1_6(byte gearPos, ushort mapTarget, ushort speed, ushort mask)
        {
            GearPos = gearPos;
            MapTarget = mapTarget;
            VehicleSpeed = speed;
            EpsEvMsk = mask;
        }

        public static ME1_6 Decode(byte[] payload)
        {
            if (payload.Length < 7)
                throw new ArgumentException("Payload too short for Frame6");

            byte gear = payload[0];
            ushort map = (ushort)((payload[2] << 8) | payload[1]);
            ushort speed = (ushort)((payload[4] << 8) | payload[3]);
            ushort mask = (ushort)((payload[6] << 8) | payload[5]);

            return new ME1_6(gear, map, speed, mask);
        }
    }
}