using ME1_4NET;
using ME1_4NET.Frames;
using Xunit;

namespace ME1_4NET.Tests
{
    public class CanDecoderTests
    {
        [Fact]
        public void Decode_ME1_1Pid_ReturnsME1_1Frame()
        {
            var result = CanDecoder.Decode(Pid.ME1_1, new byte[8]);
            Assert.IsType<ME1_1>(result);
        }

        [Fact]
        public void Decode_ME1_2Pid_ReturnsME1_2Frame()
        {
            var result = CanDecoder.Decode(Pid.ME1_2, new byte[8]);
            Assert.IsType<ME1_2>(result);
        }

        [Fact]
        public void Decode_ME1_3Pid_ReturnsME1_3Frame()
        {
            var result = CanDecoder.Decode(Pid.ME1_3, new byte[8]);
            Assert.IsType<ME1_3>(result);
        }

        [Fact]
        public void Decode_ME1_4Pid_ReturnsME1_4Frame()
        {
            var result = CanDecoder.Decode(Pid.ME1_4, new byte[7]);
            Assert.IsType<ME1_4>(result);
        }

        [Fact]
        public void Decode_ME1_5Pid_ReturnsME1_5Frame()
        {
            var result = CanDecoder.Decode(Pid.ME1_5, new byte[8]);
            Assert.IsType<ME1_5>(result);
        }

        [Fact]
        public void Decode_ME1_6Pid_ReturnsME1_6Frame()
        {
            var result = CanDecoder.Decode(Pid.ME1_6, new byte[7]);
            Assert.IsType<ME1_6>(result);
        }

        [Fact]
        public void Decode_ME1_7Pid_ReturnsME1_7Frame()
        {
            var result = CanDecoder.Decode(Pid.ME1_7, new byte[7]);
            Assert.IsType<ME1_7>(result);
        }

        [Fact]
        public void Decode_ME1_8Pid_ReturnsME1_8Frame()
        {
            var result = CanDecoder.Decode(Pid.ME1_8, new byte[8]);
            Assert.IsType<ME1_8>(result);
        }

        [Fact]
        public void Decode_UnknownPid_ThrowsKeyNotFoundException()
        {
            Assert.Throws<KeyNotFoundException>(() => CanDecoder.Decode((Pid)0xFFFF, new byte[8]));
        }

        [Fact]
        public void Decode_ME1_1Pid_DecodesPayloadValues()
        {
            // rpm = 1500 (0x05DC)
            byte[] payload = [0xDC, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
            var result = (ME1_1)CanDecoder.Decode(Pid.ME1_1, payload);
            Assert.Equal((ushort)1500, result.Rpm);
        }

        [Fact]
        public void Decode_ME1_6Pid_DecodesPayloadValues()
        {
            // gear = 4, map_target = 0x0190 = 400, vehicle_speed = 0x00C8 = 200
            byte[] payload = [0x04, 0x90, 0x01, 0xC8, 0x00, 0x00, 0x00];
            var result = (ME1_6)CanDecoder.Decode(Pid.ME1_6, payload);
            Assert.Equal((byte)4, result.GearPos);
            Assert.Equal((ushort)400, result.MapTarget);
            Assert.Equal((ushort)200, result.VehicleSpeed);
        }
    }
}
