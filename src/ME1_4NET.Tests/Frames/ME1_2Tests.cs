using Xunit;

namespace ME1_4NET.Frames.Tests
{
    public class ME1_2Test
    {
        [Fact]
        public void Decode_ValidPayload_ReturnsCorrectValues()
        {
            // Arrange
            // rpm_hard_limit = 4000 (0x0FA0), afr_curr_1 = 120, afr_curr_2 = 130,
            // lambda_trim = 2500 (0x09C4), afr_target = 14, fuel_eth_perc = 85
            byte[] payload =
            [
                0xA0, 0x0F, // rpm_hard_limit = 4000
                0x78,       // afr_curr_1 = 120
                0x82,       // afr_curr_2 = 130
                0xC4, 0x09, // lambda_trim = 2500
                0x0E,       // afr_target = 14
                0x55        // fuel_eth_perc = 85
            ];

            // Act
            ME1_2 frame = ME1_2.Decode(payload);

            // Assert
            Assert.Equal((ushort)4000, frame.RpmHardLimit);
            Assert.Equal((byte)120, frame.AfrCurr1);
            Assert.Equal((byte)130, frame.AfrCurr2);
            Assert.Equal((ushort)2500, frame.LambdaTrim);
            Assert.Equal((byte)14, frame.AfrTarget);
            Assert.Equal((byte)85, frame.FuelEthPerc);
        }

        [Fact]
        public void Decode_PayloadTooShort_ThrowsArgumentException()
        {
            // Arrange
            byte[] payload = new byte[7];

            // Act & Assert
            Assert.Throws<ArgumentException>(() => ME1_2.Decode(payload));
        }

        [Fact]
        public void Decode_EdgeValues_WorksCorrectly()
        {
            // Arrange
            // All fields set to their max values
            byte[] payload =
            [
                0xFF, 0xFF, // rpm_hard_limit = 65535
                0xFF,       // afr_curr_1 = 255
                0x00,       // afr_curr_2 = 0
                0x00, 0x00, // lambda_trim = 0
                0xFF,       // afr_target = 255
                0x00        // fuel_eth_perc = 0
            ];

            // Act
            ME1_2 frame = ME1_2.Decode(payload);

            // Assert
            Assert.Equal(ushort.MaxValue, frame.RpmHardLimit);
            Assert.Equal(byte.MaxValue, frame.AfrCurr1);
            Assert.Equal((byte)0, frame.AfrCurr2);
            Assert.Equal((ushort)0, frame.LambdaTrim);
            Assert.Equal(byte.MaxValue, frame.AfrTarget);
            Assert.Equal((byte)0, frame.FuelEthPerc);
        }
    }
}