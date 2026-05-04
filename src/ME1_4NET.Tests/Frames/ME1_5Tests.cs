using ME1_4NET.Frames;
using Xunit;

namespace ME1_4NET.Tests.Frames
{
    public class ME1_5Test
    {
        [Fact]
        public void Decode_ValidPayload_ReturnsCorrectValues()
        {
            // oil_temp raw = 900, * 0.1 = 90.0 deg C
            // oil_pressure raw = 4000, * 0.1 = 400.0 kPa
            // clt raw = 850, * 0.1 = 85.0 deg C
            // vbat raw = 138, * 0.1 = 13.8 V
            byte[] payload =
            [
                0x84, 0x03, // oil_temp raw = 900
                0xA0, 0x0F, // oil_pressure raw = 4000
                0x52, 0x03, // clt raw = 850
                0x8A, 0x00  // vbat raw = 138
            ];

            ME1_5 frame = ME1_5.Decode(payload);

            Assert.Equal(90.0f, frame.OilTemp, 1);
            Assert.Equal(400.0f, frame.OilPressure, 1);
            Assert.Equal(85.0f, frame.Clt, 1);
            Assert.Equal(13.8f, frame.Vbat, 1);
        }

        [Fact]
        public void Decode_PayloadTooShort_ThrowsArgumentException()
        {
            byte[] payload = new byte[7];
            Assert.Throws<ArgumentException>(() => ME1_5.Decode(payload));
        }

        [Fact]
        public void Decode_AllZeros_ReturnsZeros()
        {
            byte[] payload = new byte[8];
            ME1_5 frame = ME1_5.Decode(payload);

            Assert.Equal(0f, frame.OilTemp);
            Assert.Equal(0f, frame.OilPressure);
            Assert.Equal(0f, frame.Clt);
            Assert.Equal(0f, frame.Vbat);
        }

        [Fact]
        public void Decode_NegativeValues_ReturnsCorrectly()
        {
            // oil_temp raw = -200, * 0.1 = -20.0 deg C
            byte[] payload =
            [
                0x38, 0xFF, // oil_temp raw = -200
                0x00, 0x00, // oil_pressure = 0
                0x38, 0xFF, // clt raw = -200, * 0.1 = -20.0
                0x00, 0x00  // vbat = 0
            ];

            ME1_5 frame = ME1_5.Decode(payload);

            Assert.Equal(-20.0f, frame.OilTemp, 1);
            Assert.Equal(-20.0f, frame.Clt, 1);
        }
    }
}