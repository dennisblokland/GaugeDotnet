using Xunit;
using ME1_4Net.Frames;


namespace ME1_4Net.Tests.Frames
{
    public class ME1_4Test
    {
        [Fact]
        public void Decode_ValidPayload_ReturnsCorrectValues()
        {
            // Arrange
            // oilTemp = -10 (0xF6), oilPressure = 300 (0x012C), fuelPressure = 500 (0x01F4)
            byte[] payload =
            [
                0xF6,       // oilTemp = -10
                0x2C, 0x01, // oilPressure = 300
                0xF4, 0x01  // fuelPressure = 500
            ];

            // Act
            ME1_4 frame = ME1_4.Decode(payload);

            // Assert
            Assert.Equal(-10, frame.OilTemp);
            Assert.Equal((ushort)300, frame.OilPressure);
            Assert.Equal((ushort)500, frame.FuelPressure);
        }

        [Fact]
        public void Decode_PayloadTooShort_ThrowsArgumentException()
        {
            // Arrange
            byte[] payload = new byte[4];

            // Act & Assert
            Assert.Throws<ArgumentException>(() => ME1_4.Decode(payload));
        }

        [Fact]
        public void Decode_AllZeros_ReturnsZeros()
        {
            // Arrange
            byte[] payload = new byte[5];

            // Act
            ME1_4 frame = ME1_4.Decode(payload);

            // Assert
            Assert.Equal(0, frame.OilTemp);
            Assert.Equal((ushort)0, frame.OilPressure);
            Assert.Equal((ushort)0, frame.FuelPressure);
        }

        [Fact]
        public void Decode_EdgeValues_WorksCorrectly()
        {
            // Arrange
            // oilTemp = sbyte.MinValue (-128, 0x80), oilPressure = ushort.MaxValue (0xFFFF), fuelPressure = ushort.MinValue (0x0000)
            byte[] payload =
            [
                0x80,       // oilTemp = -128
                0xFF, 0xFF, // oilPressure = 65535
                0x00, 0x00  // fuelPressure = 0
            ];

            // Act
            ME1_4 frame = ME1_4.Decode(payload);

            // Assert
            Assert.Equal(sbyte.MinValue, frame.OilTemp);
            Assert.Equal(ushort.MaxValue, frame.OilPressure);
            Assert.Equal((ushort)0, frame.FuelPressure);
        }

        [Fact]
        public void Decode_MaxOilTempAndFuelPressure_WorksCorrectly()
        {
            // Arrange
            // oilTemp = sbyte.MaxValue (127, 0x7F), oilPressure = 0, fuelPressure = ushort.MaxValue (0xFFFF)
            byte[] payload =
            [
                0x7F,       // oilTemp = 127
                0x00, 0x00, // oilPressure = 0
                0xFF, 0xFF  // fuelPressure = 65535
            ];

            // Act
            ME1_4 frame = ME1_4.Decode(payload);

            // Assert
            Assert.Equal(sbyte.MaxValue, frame.OilTemp);
            Assert.Equal((ushort)0, frame.OilPressure);
            Assert.Equal(ushort.MaxValue, frame.FuelPressure);
        }
    }
}