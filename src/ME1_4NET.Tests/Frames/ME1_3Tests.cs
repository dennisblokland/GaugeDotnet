using ME1_4NET.Frames;
using Xunit;

namespace ME1_4NET.Tests.Frames
{
    public class ME1_3Tests
    {
        [Fact]
        public void Decode_ValidPayload_ReturnsCorrectValues()
        {
            // Arrange
            // ign_adv_angle = 1000 (0x03E8), ign_dwell = -2000 (0xF830), pri_inj_angle = 32767 (0x7FFF), pri_inj_pw = -32768 (0x8000)
            byte[] payload =
            [
                0xE8, 0x03, // ign_adv_angle = 1000
                0x30, 0xF8, // ign_dwell = -2000
                0xFF, 0x7F, // pri_inj_angle = 32767
                0x00, 0x80  // pri_inj_pw = -32768
            ];

            // Act
            ME1_3 frame = ME1_3.Decode(payload);

            // Assert
            Assert.Equal(1000, frame.IgnAdvAngle);
            Assert.Equal(-2000, frame.IgnDwell);
            Assert.Equal(32767, frame.PriInjAngle);
            Assert.Equal(-32768, frame.PriInjPw);
        }

        [Fact]
        public void Decode_PayloadTooShort_ThrowsArgumentException()
        {
            // Arrange
            byte[] payload = new byte[7];

            // Act & Assert
            Assert.Throws<ArgumentException>(() => ME1_3.Decode(payload));
        }

        [Fact]
        public void Decode_AllZeros_ReturnsZeros()
        {
            // Arrange
            byte[] payload = new byte[8];

            // Act
            ME1_3 frame = ME1_3.Decode(payload);

            // Assert
            Assert.Equal(0, frame.IgnAdvAngle);
            Assert.Equal(0, frame.IgnDwell);
            Assert.Equal(0, frame.PriInjAngle);
            Assert.Equal(0, frame.PriInjPw);
        }

        [Fact]
        public void Decode_MaxMinValues_WorksCorrectly()
        {
            // Arrange
            // ign_adv_angle = short.MaxValue (0x7FFF), ign_dwell = short.MinValue (0x8000), pri_inj_angle = -1 (0xFFFF), pri_inj_pw = 1 (0x0001)
            byte[] payload =
            [
                0xFF, 0x7F, // ign_adv_angle = 32767
                0x00, 0x80, // ign_dwell = -32768
                0xFF, 0xFF, // pri_inj_angle = -1
                0x01, 0x00  // pri_inj_pw = 1
            ];

            // Act
            ME1_3 frame = ME1_3.Decode(payload);

            // Assert
            Assert.Equal(short.MaxValue, frame.IgnAdvAngle);
            Assert.Equal(short.MinValue, frame.IgnDwell);
            Assert.Equal(-1, frame.PriInjAngle);
            Assert.Equal(1, frame.PriInjPw);
        }
    }
}