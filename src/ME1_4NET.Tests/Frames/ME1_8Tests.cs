using ME1_4Net.Frames;
using Xunit;

namespace ME1_4Net.Tests.Frames
{
    public class ME1_8Test
    {
        [Fact]
        public void Decode_ValidPayload_ReturnsCorrectValues()
        {
            // Arrange
            byte[] payload = [0x34, 0x12, 0xFF, 0x7F, 0xCD, 0xAB];

            // Act
            ME1_8 frame = ME1_8.Decode(payload);

            // Assert
            Assert.Equal(0x1234, frame.TpsVoltage);
            Assert.Equal(0x7FFF, frame.CamAngle);
            Assert.Equal(0xABCD, frame.Baro);
        }

        [Fact]
        public void Decode_PayloadTooShort_ThrowsArgumentException()
        {
            // Arrange
            byte[] payload = [0x01, 0x02, 0x03, 0x04, 0x05]; // Only 5 bytes

            // Act & Assert
            Assert.Throws<ArgumentException>(() => ME1_8.Decode(payload));
        }

        [Fact]
        public void Decode_MinValues_ReturnsCorrectly()
        {
            // Arrange
            byte[] payload = [0x00, 0x00, 0x00, 0x80, 0x00, 0x00];

            // Act
            ME1_8 frame = ME1_8.Decode(payload);

            // Assert
            Assert.Equal(0x0000, frame.TpsVoltage);
            Assert.Equal(-32768, frame.CamAngle);
            Assert.Equal(0x0000, frame.Baro);
        }

        [Fact]
        public void Decode_MaxValues_ReturnsCorrectly()
        {
            // Arrange
            byte[] payload = [0xFF, 0xFF, 0xFF, 0x7F, 0xFF, 0xFF];

            // Act
            ME1_8 frame = ME1_8.Decode(payload);

            // Assert
            Assert.Equal(0xFFFF, frame.TpsVoltage);
            Assert.Equal(32767, frame.CamAngle);
            Assert.Equal(0xFFFF, frame.Baro);
        }

        [Fact]
        public void Decode_NegativeCamAngle_ReturnsCorrectly()
        {
            // Arrange
            byte[] payload = [0x00, 0x00, 0xC7, 0xCF, 0x00, 0x00];

            // Act
            ME1_8 frame = ME1_8.Decode(payload);

            // Assert
            Assert.Equal(-12345, frame.CamAngle);
        }
    }
}
