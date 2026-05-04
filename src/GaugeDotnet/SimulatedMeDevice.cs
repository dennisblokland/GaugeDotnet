using ME1_4NET;

namespace GaugeDotnet
{
    /// <summary>
    /// A simulated ME device for debug/PC use. Generates oscillating gauge values so the
    /// gauge can be developed and tested without a BLE connection.
    /// </summary>
    public sealed class SimulatedMeDevice : IMeDevice, IDisposable
    {
        private readonly CancellationTokenSource _cts = new();
        private readonly Task _simulationTask;

        public MEData Data { get; } = new();
        public bool IsConnected => ConnectionState == ConnectionState.Connected;
        public ConnectionState ConnectionState { get; private set; } = ConnectionState.Disconnected;

        public event Action<IMeDevice, ConnectionState>? ConnectionStateChanged;

        public SimulatedMeDevice()
        {
            _simulationTask = Task.Run(SimulateAsync);
        }

        public Task ConnectAsync()
        {
            ConnectionState = ConnectionState.Connected;
            ConnectionStateChanged?.Invoke(this, ConnectionState);
            Console.WriteLine("[SimulatedMeDevice] Connected (simulated)");
            return Task.CompletedTask;
        }

        private async Task SimulateAsync()
        {
            double t = 0;
            while (!_cts.IsCancellationRequested)
            {
                double sine = Math.Sin(t);
                double fast = Math.Sin(t * 3);

                // ME1_1
                Data.Rpm = (ushort)(3500 + sine * 3000);
                Data.ThrottlePosition = (float)(50 + sine * 50);
                Data.Map = (float)(50 + sine * 50);
                Data.Iat = (float)(35 + fast * 10);

                // ME1_2
                Data.AfrCurr1 = (float)Math.Round(14.5 + sine * 2.5, 2);
                Data.AfrCurr2 = (float)Math.Round(14.5 + sine * 2.3, 2);
                Data.AfrTarget = (float)Math.Round(14.7 + fast * 0.5, 2);
                Data.FuelEthPerc = 0;

                // ME1_3
                Data.IgnAdvAngle = (short)(20 + sine * 15);
                Data.IgnDwell = (short)(30 + fast * 5);
                Data.PriInjPw = (short)(50 + sine * 30);

                // ME1_4
                Data.PriInjDuty = (float)(40 + sine * 35);
                Data.SecInjDuty = (float)(20 + sine * 15);
                Data.SecInjAngle = (float)(350 + fast * 10);
                Data.SecInjPw = (float)(5 + sine * 3);
                Data.BoostCtrlDuty = (float)(50 + sine * 40);

                // ME1_5
                Data.OilTemp = (float)(90 + fast * 15);
                Data.OilPressure = (float)(400 + sine * 200);
                Data.Clt = (float)(85 + fast * 10);
                Data.Vbat = (float)(13.8 + fast * 0.5);

                // ME1_6
                Data.GearPos = (byte)Math.Clamp((int)(3 + sine * 2.5), 1, 6);
                Data.VehicleSpeed = (ushort)(60 + sine * 60);

                // ME1_7
                Data.KnockPeakReading = (ushort)(100 + sine * 50);
                Data.KnockIgnAdvMod = (float)(sine * -2.0);
                Data.FuelPressure = (ushort)(400 + sine * 100);
                Data.FuelTemp = (byte)(40 + fast * 10);
                Data.KnockEvsCnt = (ushort)(fast * 5);

                // ME1_8
                Data.Egt1 = (float)(700 + sine * 200);
                Data.Egt2 = (float)(680 + sine * 180);
                Data.Gpt1 = (short)(sine * 20);
                Data.Gpt2 = (short)(sine * 15);

                t += 0.05;

                try
                {
                    await Task.Delay(50, _cts.Token);
                }
                catch (OperationCanceledException)
                {
                    break;
                }
            }
        }

        public void Dispose()
        {
            _cts.Cancel();
            _simulationTask.Wait(500);
            _cts.Dispose();
        }
    }
}
