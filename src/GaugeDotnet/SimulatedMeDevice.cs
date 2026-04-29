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
                Data.AfrCurr1 = (decimal)Math.Round(14.5 + sine * 2.5, 2);
                Data.AfrCurr2 = (decimal)Math.Round(14.5 + sine * 2.3, 2);
                Data.AfrTarget = (decimal)Math.Round(14.7 + fast * 0.5, 2);
                Data.FuelEthPerc = 0;

                // ME1_3
                Data.IgnAdvAngle = (short)(20 + sine * 15);
                Data.IgnDwell = (short)(30 + fast * 5);
                Data.PriInjPw = (short)(50 + sine * 30);

                // ME1_4
                Data.OilTemp = (sbyte)(90 + fast * 15);
                Data.OilPressure = (ushort)(40 + sine * 20);
                Data.FuelPressure = (ushort)(43 + fast * 3);

                // ME1_5
                Data.BatteryVoltage = (ushort)(138 + fast * 5);
                Data.KnockLevel = (byte)Math.Max(0, fast * 3);
                Data.InjectorDuty = (byte)(40 + sine * 35);

                // ME1_6
                Data.GearPos = (byte)Math.Clamp((int)(3 + sine * 2.5), 1, 6);
                Data.VehicleSpeed = (ushort)(60 + sine * 60);

                // ME1_7
                Data.Egt = (ushort)(700 + sine * 200);
                Data.TurboSpeed = (ushort)(80000 + sine * 40000);
                Data.WastegateDuty = (byte)(50 + sine * 40);

                // ME1_8
                Data.Baro = (ushort)(1013 + fast * 5);
                Data.CamAngle = (short)(sine * 20);

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
