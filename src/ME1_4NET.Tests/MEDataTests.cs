using ME1_4NET;
using ME1_4NET.Frames;
using Xunit;

namespace ME1_4NET.Tests
{
    public class MEDataTests
    {
        [Fact]
        public void Apply_ME1_1Frame_UpdatesProperties()
        {
            var data = new MEData();
            // rpm=3000, throttle raw=1000 →100.0, map raw=500 →5.0, iat raw=250 →25.0
            byte[] payload = [0xB8, 0x0B, 0xE8, 0x03, 0xF4, 0x01, 0xFA, 0x00];
            data.Apply(ME1_1.Decode(payload));

            Assert.Equal((ushort)3000, data.Rpm);
            Assert.Equal(100.0f, data.ThrottlePosition, 1);
            Assert.Equal(5.0f, data.Map, 2);
            Assert.Equal(25.0f, data.Iat, 1);
        }

        [Fact]
        public void Apply_ME1_2Frame_UpdatesProperties()
        {
            var data = new MEData();
            // rpmHard=4000, afr1=13.5, afr2=14.0, lambdaTrim=2500, afrTarget=14, fuelEth=85
            byte[] payload = [0xA0, 0x0F, 0x78, 0x82, 0xC4, 0x09, 0x0E, 0x55];
            data.Apply(ME1_2.Decode(payload));

            Assert.Equal(4000, data.RpmHardLimit);
            Assert.Equal(13.5f, data.AfrCurr1);
            Assert.Equal(2500, data.LambdaTrim);
            Assert.Equal(85f, data.FuelEthPerc);
        }

        [Fact]
        public void Apply_ME1_3Frame_UpdatesProperties()
        {
            var data = new MEData();
            // ignAdv=1000, dwell=-2000, priAngle=32767, priPw=-32768
            byte[] payload = [0xE8, 0x03, 0x30, 0xF8, 0xFF, 0x7F, 0x00, 0x80];
            data.Apply(ME1_3.Decode(payload));

            Assert.Equal((short)1000, data.IgnAdvAngle);
            Assert.Equal((short)-2000, data.IgnDwell);
            Assert.Equal(short.MaxValue, data.PriInjAngle);
            Assert.Equal(short.MinValue, data.PriInjPw);
        }

        [Fact]
        public void Apply_ME1_4Frame_UpdatesProperties()
        {
            var data = new MEData();
            // priDuty=50, secDuty=30, secAngle=350, secPw=5, boostDuty=70
            byte[] payload = [100, 60, 0xAC, 0x0D, 0x32, 0x00, 140];
            data.Apply(ME1_4.Decode(payload));

            Assert.Equal(50.0f, data.PriInjDuty);
            Assert.Equal(30.0f, data.SecInjDuty);
            Assert.Equal(350.0f, data.SecInjAngle, 1);
            Assert.Equal(5.0f, data.SecInjPw, 1);
            Assert.Equal(70.0f, data.BoostCtrlDuty);
        }

        [Fact]
        public void Apply_ME1_5Frame_UpdatesProperties()
        {
            var data = new MEData();
            // oilTemp=90, oilPressure=400, clt=85, vbat=13.8
            byte[] payload = [0x84, 0x03, 0xA0, 0x0F, 0x52, 0x03, 0x8A, 0x00];
            data.Apply(ME1_5.Decode(payload));

            Assert.Equal(90.0f, data.OilTemp, 1);
            Assert.Equal(400.0f, data.OilPressure, 1);
            Assert.Equal(85.0f, data.Clt, 1);
            Assert.Equal(13.8f, data.Vbat, 1);
        }

        [Fact]
        public void Apply_ME1_6Frame_UpdatesProperties()
        {
            var data = new MEData();
            // gear=3, mapTarget=0x0050=80, speed=0x0190=400, mask=0
            byte[] payload = [0x03, 0x50, 0x00, 0x90, 0x01, 0x00, 0x00];
            data.Apply(ME1_6.Decode(payload));

            Assert.Equal((byte)3, data.GearPos);
            Assert.Equal((ushort)80, data.MapTarget);
            Assert.Equal((ushort)400, data.VehicleSpeed);
            Assert.Equal((ushort)0, data.EpsEvMsk);
        }

        [Fact]
        public void Apply_ME1_7Frame_UpdatesProperties()
        {
            var data = new MEData();
            // knockPeak=0x1234, knockIgnAdv=-2.0, fuelPressure=400, fuelTemp=45, knockEvs=5
            byte[] payload = [0x34, 0x12, 0xEC, 80, 45, 0x05, 0x00];
            data.Apply(ME1_7.Decode(payload));

            Assert.Equal((ushort)0x1234, data.KnockPeakReading);
            Assert.Equal(-2.0f, data.KnockIgnAdvMod, 1);
            Assert.Equal((ushort)400, data.FuelPressure);
            Assert.Equal((byte)45, data.FuelTemp);
            Assert.Equal((ushort)5, data.KnockEvsCnt);
        }

        [Fact]
        public void Apply_ME1_8Frame_UpdatesProperties()
        {
            var data = new MEData();
            // egt1=700.0, egt2=650.0, gpt1=100, gpt2=-50
            byte[] payload = [0x58, 0x1B, 0x64, 0x19, 0x64, 0x00, 0xCE, 0xFF];
            data.Apply(ME1_8.Decode(payload));

            Assert.Equal(700.0f, data.Egt1, 1);
            Assert.Equal(650.0f, data.Egt2, 1);
            Assert.Equal((short)100, data.Gpt1);
            Assert.Equal((short)-50, data.Gpt2);
        }

        [Fact]
        public void Apply_MultipleFrameTypes_EachGroupPreserved()
        {
            var data = new MEData();

            // Apply ME1_1 (rpm=3000)
            byte[] me1Payload = [0xB8, 0x0B, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
            data.Apply(ME1_1.Decode(me1Payload));

            // Apply ME1_5 (oilTemp=90)
            byte[] me5Payload = [0x84, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
            data.Apply(ME1_5.Decode(me5Payload));

            Assert.Equal((ushort)3000, data.Rpm);
            Assert.Equal(90.0f, data.OilTemp, 1);
        }

        [Fact]
        public void Apply_SameFrameTwice_OverwritesPreviousValues()
        {
            var data = new MEData();

            byte[] firstPayload = [0xB8, 0x0B, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]; // rpm=3000
            data.Apply(ME1_1.Decode(firstPayload));
            Assert.Equal((ushort)3000, data.Rpm);

            byte[] secondPayload = [0xD0, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]; // rpm=2000
            data.Apply(ME1_1.Decode(secondPayload));
            Assert.Equal((ushort)2000, data.Rpm);
        }
    }
}
