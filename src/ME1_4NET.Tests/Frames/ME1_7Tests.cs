using ME1_4NET.Frames;
using Xunit;

namespace ME1_4NET.Tests.Frames
{
    public class ME1_7Test
    {
        [Fact]
        public void Decode_ValidPayload_ReturnsCorrectValues()
        {
            // Arrange
            // knock_peak = 0x1234 = 4660
            // knock_ign_adv_mod raw = -20 (0xEC), * 0.1 = -2.0 deg
            // fuel_pressure raw = 80, * 5 = 400 kPa
            // fuel_temp = 45 deg C
            // knock_evs_cnt = 0x0005 = 5
            byte[] payload = [0x34, 0x12, 0xEC, 80, 45, 0x05, 0x00];

            // Act
            ME1_7 frame = ME1_7.Decode(payload);

            // Assert
            Assert.Equal((ushort)0x1234, frame.KnockPeakReading);
            Assert.Equal(-2.0f, frame.KnockIgnAdvMod, 1);
            Assert.Equal((ushort)400, frame.FuelPressure);
            Assert.Equal((byte)45, frame.FuelTemp);
            Assert.Equal((ushort)5, frame.KnockEvsCnt);
        }

        [Fact]
        public void Decode_PayloadTooShort_ThrowsArgumentException()
        {
            // Arrange
            byte[] payload = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06]; // Only 6 bytes

            // Act & Assert
            Assert.Throws<ArgumentException>(() => ME1_7.Decode(payload));
        }

        [Fact]
        public void Decode_AllZeros_ReturnsCorrectly()
        {
            // Arrange
            byte[] payload = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

            // Act
            ME1_7 frame = ME1_7.Decode(payload);

            // Assert
            Assert.Equal((ushort)0, frame.KnockPeakReading);
            Assert.Equal(0f, frame.KnockIgnAdvMod);
            Assert.Equal((ushort)0, frame.FuelPressure);
            Assert.Equal((byte)0, frame.FuelTemp);
            Assert.Equal((ushort)0, frame.KnockEvsCnt);
        }

        [Fact]
        public void Decode_MaxValues_ReturnsCorrectly()
        {
            // Arrange
            // knock_peak = 0xFFFF, knock_ign raw = 127 (max i8), * 0.1 = 12.7
            // fuel_press raw = 255, * 5 = 1275
            // fuel_temp = 255, knock_evs = 0xFFFF
            byte[] payload = [0xFF, 0xFF, 0x7F, 0xFF, 0xFF, 0xFF, 0xFF];

            // Act
            ME1_7 frame = ME1_7.Decode(payload);

            // Assert
            Assert.Equal(ushort.MaxValue, frame.KnockPeakReading);
            Assert.Equal(12.7f, frame.KnockIgnAdvMod, 1);
            Assert.Equal((ushort)1275, frame.FuelPressure);
            Assert.Equal(byte.MaxValue, frame.FuelTemp);
            Assert.Equal(ushort.MaxValue, frame.KnockEvsCnt);
        }
    }
}
