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
            byte[] payload = [0x34, 0x12, 0xCD, 0xAB, 0x56];

            // Act
            ME1_7 frame = ME1_7.Decode(payload);

            // Assert
            Assert.Equal(0x1234, frame.Egt);
            Assert.Equal(0xABCD, frame.TurboSpeed);
            Assert.Equal(0x56, frame.WastegateDuty);
        }

        [Fact]
        public void Decode_PayloadTooShort_ThrowsArgumentException()
        {
            // Arrange
            byte[] payload = [0x01, 0x02, 0x03, 0x04]; // Only 4 bytes

            // Act & Assert
            Assert.Throws<ArgumentException>(() => ME1_7.Decode(payload));
        }

        [Fact]
        public void Decode_MinValues_ReturnsCorrectly()
        {
            // Arrange
            byte[] payload = [0x00, 0x00, 0x00, 0x00, 0x00];

            // Act
            ME1_7 frame = ME1_7.Decode(payload);

            // Assert
            Assert.Equal(0x0000, frame.Egt);
            Assert.Equal(0x0000, frame.TurboSpeed);
            Assert.Equal(0x00, frame.WastegateDuty);
        }

        [Fact]
        public void Decode_MaxValues_ReturnsCorrectly()
        {
            // Arrange
            byte[] payload = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF];

            // Act
            ME1_7 frame = ME1_7.Decode(payload);

            // Assert
            Assert.Equal(0xFFFF, frame.Egt);
            Assert.Equal(0xFFFF, frame.TurboSpeed);
            Assert.Equal(0xFF, frame.WastegateDuty);
        }
    }
}
