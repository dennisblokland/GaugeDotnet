using ME1_4NET.Frames;

namespace ME1_4NET
{
    public static class Examples
    {
        public static void Run()
        {
            byte pidByte = 0x10;
            byte[] payload = new byte[] { 0x07, 0xD0, 0x07, 0xD0, 0x00, 0x64, 0x00, 0x50 };
            Pid pid = (Pid)pidByte;
            ICanFrame frame = CanDecoder.Decode(pid, payload);

            if (frame is ME1_1 f1)
            {
                Console.WriteLine($"RPM: {f1.Rpm}");
                Console.WriteLine($"Throttle: {f1.ThrottlePosition}");
                Console.WriteLine($"MAP: {f1.Map}");
                Console.WriteLine($"IAT: {f1.Iat}");
            }
        }
    }
}
