namespace GaugeDotnet
{
    /// <summary>
    /// A simulated ME device for debug/PC use. Generates oscillating AFR values so the
    /// gauge can be developed and tested without a BLE connection.
    /// </summary>
    public sealed class SimulatedMeDevice : IMeDevice, IDisposable
    {
        private readonly CancellationTokenSource _cts = new();
        private readonly Task _simulationTask;

        public decimal afr { get; private set; } = 14.7m;
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
            // Oscillate AFR between 12 and 17 in a smooth sine wave pattern
            double t = 0;
            while (!_cts.IsCancellationRequested)
            {
                double sine = Math.Sin(t);          // -1 to 1
                double value = 14.5 + sine * 2.5;   // 12.0 to 17.0
                afr = (decimal)Math.Round(value, 2);
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
