using ME1_4NET;
using VaettirNet.Btleplug;

namespace GaugeDotnet
{
    public class MeDevice : IDisposable, IMeDevice
    {
        private readonly BtlePeripheral _ble;
        private bool _shouldTryReconnect;
        public ConnectionState ConnectionState { get; private set; }
        public bool IsConnected => ConnectionState == ConnectionState.Connected;

        public event Action<IMeDevice, ConnectionState>? ConnectionStateChanged;
        private readonly object _disconnectLock = new();
        private Task? _reconnectTask;
        private CancellationTokenSource? _reconnectCancel;
        public MEData Data { get; } = new();
        private static readonly byte[] MagicAllPidPackage =
                        [
                            0x01,
                                (50 >> 8),
                                (50 & 0xFF)
                        ];

        public MeDevice(BtlePeripheral ble)
        {
            _ble = ble;
            ble.Disconnected += Disconnected;
        }

        private void Disconnected(BtlePeripheral obj)
        {
            if (!_shouldTryReconnect || _reconnectTask != null)
            {
                SetConnectionState(ConnectionState.Disconnected);
                return;
            }

            lock (_disconnectLock)
            {
                if (!_shouldTryReconnect || _reconnectTask != null)
                {
                    SetConnectionState(ConnectionState.Disconnected);
                    return;
                }

                Console.WriteLine($"Lost device {obj.Address}, attempting reconnect");
                SetConnectionState(ConnectionState.Reconnecting);
                _reconnectCancel = new CancellationTokenSource();
                _reconnectTask = Task.Run(() => AttemptReconnect(_reconnectCancel.Token));
            }
        }

        private void SetConnectionState(ConnectionState conn)
        {
            ConnectionState = conn;
            ConnectionStateChanged?.Invoke(this, conn);
        }
        private async Task AttemptReconnect(CancellationToken cancellationToken)
        {
            int delaySeconds = 1;
            while (true)
            {
                try
                {
                    if (await _ble.IsConnectedAsync())
                    {
                        SetConnectionState(ConnectionState.Connected);
                        return;
                    }

                    await _ble.ConnectAsync();
                    Console.WriteLine($"Device {_ble.Address} reconnected");
                    lock (_disconnectLock)
                    {
                        _reconnectCancel = null;
                        _reconnectTask = null;
                    }

                    return;
                }
                catch
                {
                    Console.WriteLine($"Could not reconnect {_ble.Address}, trying again in {delaySeconds} seconds");
                    await Task.Delay(TimeSpan.FromSeconds(delaySeconds), cancellationToken);
                    delaySeconds = int.Min(10, delaySeconds + 1);
                }
            }
        }
        internal static MeDevice Create(BtlePeripheral ble)
        {
            return new MeDevice(ble);
        }

        public async Task ConnectAsync()
        {
            _shouldTryReconnect = true;
            await _ble.ConnectAsync().ConfigureAwait(false);
            // Retrieve the list of services to ensure the device is fully initialized before proceeding
            await _ble.GetServicesAsync();
            await _ble.RegisterNotificationCallback(RaceChronoIds.ServiceUuid, RaceChronoIds.CanBusCharacteristicUuid, DataReceived);
            await SendMessage(MagicAllPidPackage).ConfigureAwait(false);
        }
        private async Task SendMessage(byte[] msg)
        {
            await _ble.Write(
                RaceChronoIds.ServiceUuid,
                RaceChronoIds.PidCharacteristicUuid,
                msg,
                true
            );
        }

        private void DataReceived(BtlePeripheral peripheral, Guid service, Guid characteristic, Span<byte> data)
        {
            try
            {
                // get can id form the first two bytes of data
                if (data.Length < 2)
                {
                    return;
                }

                ushort canId = BitConverter.ToUInt16(data.ToArray(), 0);
                ReadOnlySpan<byte> dataPacket = data[4..];

                Pid pid = (Pid)(canId & 0x0FFF); // Assuming the PID is in the lower 12 bits of the CAN ID
                if (!Enum.IsDefined(pid))
                {
                    //    Console.WriteLine($"Unknown PID: {pid}");
                    return;
                }

                ICanFrame frame = CanDecoder.Decode(pid, dataPacket.ToArray());
                Data.Apply(frame);
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Exception in DataReceived: {ex}");
            }
        }

        public void Dispose()
        {
            DisposeAsync().AsTask().GetAwaiter().GetResult();
        }

        public async ValueTask DisposeAsync()
        {
            _reconnectCancel?.Cancel();
            if (ConnectionState == ConnectionState.Connected)
                await _ble.DisconnectAsync();
            _ble.Dispose();
        }
    }
}