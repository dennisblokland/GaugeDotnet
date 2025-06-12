using ME1_4Net.Frames;

namespace ME1_4Net
{
    /// <summary>
    /// Dispatches raw CAN payloads to their strongly-typed frame structs.
    /// </summary>
    public static class CanDecoder
    {
        private static readonly Dictionary<Pid, Func<byte[], ICanFrame>> Decoders =
            new Dictionary<Pid, Func<byte[], ICanFrame>>
        {
            { Pid.ME1_1, data => ME1_1.Decode(data) },
            { Pid.ME1_2, data => ME1_2.Decode(data) },
            { Pid.ME1_3, data => ME1_3.Decode(data) },
            { Pid.ME1_4, data => ME1_4.Decode(data) },
            { Pid.ME1_5, data => ME1_5.Decode(data) },
            { Pid.ME1_6, data => ME1_6.Decode(data) },
            { Pid.ME1_7, data => ME1_7.Decode(data) },
            { Pid.ME1_8, data => ME1_8.Decode(data) },
        };

        public static ICanFrame Decode(Pid pid, byte[] payload)
        {
            if (!Decoders.TryGetValue(pid, out Func<byte[], ICanFrame>? decoder))
                throw new KeyNotFoundException($"Unsupported PID: {pid}");

            return decoder(payload);
        }
    }
}
