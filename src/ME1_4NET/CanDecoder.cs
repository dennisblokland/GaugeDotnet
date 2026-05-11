using ME1_4NET.Frames;

namespace ME1_4NET
{
    public static class CanDecoder
    {
        public static ICanFrame Decode(Pid pid, ReadOnlySpan<byte> payload) => pid switch
        {
            Pid.ME1_1 => ME1_1.Decode(payload),
            Pid.ME1_2 => ME1_2.Decode(payload),
            Pid.ME1_3 => ME1_3.Decode(payload),
            Pid.ME1_4 => ME1_4.Decode(payload),
            Pid.ME1_5 => ME1_5.Decode(payload),
            Pid.ME1_6 => ME1_6.Decode(payload),
            Pid.ME1_7 => ME1_7.Decode(payload),
            Pid.ME1_8 => ME1_8.Decode(payload),
            _ => throw new KeyNotFoundException($"Unsupported PID: {pid}"),
        };
    }
}
