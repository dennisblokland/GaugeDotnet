using ME1_4NET.Frames;
using Xunit;

namespace ME1_4NET.Tests.Frames
{
    public class ME1_8Test
    {
        [Fact]
        public void Decode_ValidPayload_ReturnsCorrectValues()
        {
            // Arrange
            // egt1 raw = 7000, * 0.1 = 700.0 deg C
            // egt2 raw = 6500, * 0.1 = 650.0 deg C
            // gpt1 = 100, gpt2 = -50
            byte[] payload =
            [
                0x58, 0x1B, // egt1 raw = 7000
                0x64, 0x19, // egt2 raw = 6500
                0x64, 0x00, // gpt1 = 100
                0xCE, 0xFF  // gpt2 = -50
            ];

            // Act
            ME1_8 frame = ME1_8.Decode(payload);

            // Assert
            Assert.Equal(700.0f, frame.Egt1, 1);
            Assert.Equal(650.0f, frame.Egt2, 1);
            Assert.Equal((short)100, frame.Gpt1);
            Assert.Equal((short)-50, frame.Gpt2);
        }

        [Fact]
        public void Decode_PayloadTooShort_ThrowsArgumentException()
        {
            // Arrange
            byte[] payload = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]; // Only 7 bytes

            // Act & Assert
            Assert.Throws<ArgumentException>(() => ME1_8.Decode(payload));
        }

        [Fact]
        public void Decode_AllZeros_ReturnsCorrectly()
        {
            // Arrange
            byte[] payload = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

            // Act
            ME1_8 frame = ME1_8.Decode(payload);

            // Assert
            Assert.Equal(0f, frame.Egt1);
            Assert.Equal(0f, frame.Egt2);
            Assert.Equal((short)0, frame.Gpt1);
            Assert.Equal((short)0, frame.Gpt2);
        }

        [Fact]
        public void Decode_MaxEgt_ReturnsCorrectly()
        {
            // Arrange
            // egt1 raw = 0xFFFF = 65535, * 0.1 = 6553.5
            byte[] payload = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F, 0x00, 0x80];

            // Act
            ME1_8 frame = ME1_8.Decode(payload);

            // Assert
            Assert.Equal(6553.5f, frame.Egt1, 1);
            Assert.Equal(6553.5f, frame.Egt2, 1);
            Assert.Equal(short.MaxValue, frame.Gpt1);
            Assert.Equal(short.MinValue, frame.Gpt2);
        }
    }
}
