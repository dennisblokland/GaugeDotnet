using ME1_4NET.Frames;
using Xunit;

namespace ME1_4NET.Tests.Frames
{
    public class ME1_1Test
    {
        [Fact]
        public void Decode_ValidPayload_ReturnsCorrectValues()
        {
            // Arrange
            // rpm = 3000 (0x0BB8), throttle = -1234 (0xFB2E), map = 2500 (0x09C4), iat = 567 (0x0237)
            byte[] payload =
            [
                0xB8, 0x0B, // rpm = 3000
                0x2E, 0xFB, // throttle = -1234
                0xC4, 0x09, // map = 2500
                0x37, 0x02  // iat = 567
            ];

            // Act
            ME1_1 frame = ME1_1.Decode(payload);

            // Assert
            Assert.Equal((ushort)3000, frame.Rpm);
            Assert.Equal(-123.4f, frame.ThrottlePosition, 1); // -1234 * 0.1
            Assert.Equal(25.00f, frame.Map, 2); // 2500 * 0.01
            Assert.Equal(56.7f, frame.Iat, 1); // 567 * 0.1
        }

        [Fact]
        public void Decode_PayloadTooShort_ThrowsArgumentException()
        {
            // Arrange
            byte[] payload = new byte[7];

            // Act & Assert
            Assert.Throws<ArgumentException>(() => ME1_1.Decode(payload));
        }

        [Fact]
        public void Decode_NegativeIatAndThrottle_WorksCorrectly()
        {
            // Arrange
            // throttle = -100 (0xFF9C), iat = -200 (0xFF38)
            byte[] payload =
            [
                0x00, 0x00, // rpm = 0
                0x9C, 0xFF, // throttle = -100
                0x00, 0x00, // map = 0
                0x38, 0xFF  // iat = -200
            ];

            // Act
            ME1_1 frame = ME1_1.Decode(payload);

            // Assert
            Assert.Equal(0, frame.Rpm);
            Assert.Equal(-10.0f, frame.ThrottlePosition, 1); // -100 * 0.1
            Assert.Equal(0.0f, frame.Map, 2);
            Assert.Equal(-20.0f, frame.Iat, 1); // -200 * 0.1
        }
    }
}