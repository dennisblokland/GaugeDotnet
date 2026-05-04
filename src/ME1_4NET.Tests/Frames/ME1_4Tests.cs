using Xunit;
using ME1_4NET.Frames;


namespace ME1_4NET.Tests.Frames
{
    public class ME1_4Test
    {
        [Fact]
        public void Decode_ValidPayload_ReturnsCorrectValues()
        {
            // Arrange
            // priInjDuty = 100 * 0.5 = 50%, secInjDuty = 60 * 0.5 = 30%
            // secInjAngle = 3500 * 0.1 = 350 deg, secInjPw = 50 * 0.1 = 5 ms
            // boostCtrlDuty = 140 * 0.5 = 70%
            byte[] payload =
            [
                100,        // priInjDuty raw = 100
                60,         // secInjDuty raw = 60
                0xAC, 0x0D, // secInjAngle raw = 3500 (little-endian)
                0x32, 0x00, // secInjPw raw = 50 (little-endian)
                140         // boostCtrlDuty raw = 140
            ];

            // Act
            ME1_4 frame = ME1_4.Decode(payload);

            // Assert
            Assert.Equal(50.0f, frame.PriInjDuty);
            Assert.Equal(30.0f, frame.SecInjDuty);
            Assert.Equal(350.0f, frame.SecInjAngle, 1);
            Assert.Equal(5.0f, frame.SecInjPw, 1);
            Assert.Equal(70.0f, frame.BoostCtrlDuty);
        }

        [Fact]
        public void Decode_PayloadTooShort_ThrowsArgumentException()
        {
            // Arrange
            byte[] payload = new byte[6];

            // Act & Assert
            Assert.Throws<ArgumentException>(() => ME1_4.Decode(payload));
        }

        [Fact]
        public void Decode_AllZeros_ReturnsZeros()
        {
            // Arrange
            byte[] payload = new byte[7];

            // Act
            ME1_4 frame = ME1_4.Decode(payload);

            // Assert
            Assert.Equal(0f, frame.PriInjDuty);
            Assert.Equal(0f, frame.SecInjDuty);
            Assert.Equal(0f, frame.SecInjAngle);
            Assert.Equal(0f, frame.SecInjPw);
            Assert.Equal(0f, frame.BoostCtrlDuty);
        }

        [Fact]
        public void Decode_MaxDuty_ReturnsCorrectValues()
        {
            // Arrange
            // 255 * 0.5 = 127.5%
            byte[] payload =
            [
                0xFF,       // priInjDuty = 127.5
                0xFF,       // secInjDuty = 127.5
                0x00, 0x00, // secInjAngle = 0
                0x00, 0x00, // secInjPw = 0
                0xFF        // boostCtrlDuty = 127.5
            ];

            // Act
            ME1_4 frame = ME1_4.Decode(payload);

            // Assert
            Assert.Equal(127.5f, frame.PriInjDuty);
            Assert.Equal(127.5f, frame.SecInjDuty);
            Assert.Equal(127.5f, frame.BoostCtrlDuty);
        }

        [Fact]
        public void Decode_NegativeAngle_ReturnsCorrectValues()
        {
            // Arrange
            // secInjAngle raw = -100, * 0.1 = -10.0 deg
            byte[] payload =
            [
                0x00,       // priInjDuty = 0
                0x00,       // secInjDuty = 0
                0x9C, 0xFF, // secInjAngle raw = -100 (little-endian)
                0x9C, 0xFF, // secInjPw raw = -100, * 0.1 = -10.0 ms
                0x00        // boostCtrlDuty = 0
            ];

            // Act
            ME1_4 frame = ME1_4.Decode(payload);

            // Assert
            Assert.Equal(-10.0f, frame.SecInjAngle, 1);
            Assert.Equal(-10.0f, frame.SecInjPw, 1);
        }
    }
}