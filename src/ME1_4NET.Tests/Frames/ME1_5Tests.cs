using ME1_4Net.Frames;
using Xunit;

namespace ME1_4Net.Tests.Frames
{
    public class ME1_5Test
    {
        [Fact]
        public void Decode_ValidPayload_ReturnsCorrectValues()
        {
            // battery_voltage = 12345 (0x3039), knock_level = 200, injector_duty = 150
            byte[] payload =
            [
                0x39, 0x30, // battery_voltage = 0x3039 = 12345
                0xC8,       // knock_level = 200
                0x96        // injector_duty = 150
            ];

            ME1_5 frame = ME1_5.Decode(payload);

            Assert.Equal((ushort)12345, frame.BatteryVoltage);
            Assert.Equal((byte)200, frame.KnockLevel);
            Assert.Equal((byte)150, frame.InjectorDuty);
        }

        [Fact]
        public void Decode_PayloadTooShort_ThrowsArgumentException()
        {
            byte[] payload = new byte[3];
            Assert.Throws<ArgumentException>(() => ME1_5.Decode(payload));
        }

        [Fact]
        public void Decode_AllZeros_ReturnsZeros()
        {
            byte[] payload = new byte[4];
            ME1_5 frame = ME1_5.Decode(payload);

            Assert.Equal((ushort)0, frame.BatteryVoltage);
            Assert.Equal((byte)0, frame.KnockLevel);
            Assert.Equal((byte)0, frame.InjectorDuty);
        }

        [Fact]
        public void Decode_EdgeValues_WorksCorrectly()
        {
            // battery_voltage = ushort.MaxValue (0xFFFF), knock_level = byte.MaxValue (0xFF), injector_duty = byte.MinValue (0x00)
            byte[] payload =
            [
                0xFF, 0xFF, // battery_voltage = 65535
                0xFF,       // knock_level = 255
                0x00        // injector_duty = 0
            ];

            ME1_5 frame = ME1_5.Decode(payload);

            Assert.Equal(ushort.MaxValue, frame.BatteryVoltage);
            Assert.Equal(byte.MaxValue, frame.KnockLevel);
            Assert.Equal((byte)0, frame.InjectorDuty);
        }

        [Fact]
        public void Decode_MinBatteryVoltageAndMaxInjectorDuty_WorksCorrectly()
        {
            // battery_voltage = 0, knock_level = 0, injector_duty = 255
            byte[] payload =
            [
                0x00, 0x00, // battery_voltage = 0
                0x00,       // knock_level = 0
                0xFF        // injector_duty = 255
            ];

            ME1_5 frame = ME1_5.Decode(payload);

            Assert.Equal((ushort)0, frame.BatteryVoltage);
            Assert.Equal((byte)0, frame.KnockLevel);
            Assert.Equal(byte.MaxValue, frame.InjectorDuty);
        }
    }
}