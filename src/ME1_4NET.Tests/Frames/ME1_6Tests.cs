using ME1_4NET.Frames;
using Xunit;

namespace ME1_4NET.Tests.Frames
{
    public class ME1_6Test
    {
        [Fact]
        public void Decode_ValidPayload_ReturnsCorrectValues()
        {
            // Arrange
            byte[] payload = [0x12, 0x56, 0x34, 0x9A, 0x78, 0xDE, 0xBC];

            // Act
            ME1_6 frame = ME1_6.Decode(payload);

            // Assert
            Assert.Equal(0x12, frame.GearPos);
            Assert.Equal(0x3456, frame.MapTarget);
            Assert.Equal(0x789A, frame.VehicleSpeed);
            Assert.Equal(0xBCDE, frame.EpsEvMsk);
        }

        [Fact]
        public void Decode_PayloadTooShort_ThrowsArgumentException()
        {
            // Arrange
            byte[] payload = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06];

            // Act & Assert
            Assert.Throws<ArgumentException>(() => ME1_6.Decode(payload));
        }

        [Fact]
        public void Decode_MinValues_ReturnsCorrectly()
        {
            // Arrange
            byte[] payload = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

            // Act
            ME1_6 frame = ME1_6.Decode(payload);

            // Assert
            Assert.Equal(0x00, frame.GearPos);
            Assert.Equal(0x0000, frame.MapTarget);
            Assert.Equal(0x0000, frame.VehicleSpeed);
            Assert.Equal(0x0000, frame.EpsEvMsk);
        }

        [Fact]
        public void Decode_MaxValues_ReturnsCorrectly()
        {
            // Arrange
            byte[] payload = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];

            // Act
            ME1_6 frame = ME1_6.Decode(payload);

            // Assert
            Assert.Equal(0xFF, frame.GearPos);
            Assert.Equal(0xFFFF, frame.MapTarget);
            Assert.Equal(0xFFFF, frame.VehicleSpeed);
            Assert.Equal(0xFFFF, frame.EpsEvMsk);
        }
    }
}
