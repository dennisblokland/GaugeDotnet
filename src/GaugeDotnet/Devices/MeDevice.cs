using ME1_4NET;
using VaettirNet.Btleplug;

namespace GaugeDotnet.Devices
{
    public class MeDevice : IDisposable, IMeDevice
    {
        private readonly BtlePeripheral _ble;
        private volatile bool _shouldTryReconnect;
        private bool _notificationsRegistered;
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
                    if (!await _ble.IsConnectedAsync())
                    {
                        await _ble.ConnectAsync();
                    }

                    // Peripheral clears its CCCD subscription and stops streaming on
                    // disconnect, so we must re-subscribe and re-send the ALLOW ALL PIDS
                    // command on every reconnect — not just the first connect.
                    await InitializeSessionAsync();
                    Console.WriteLine($"Device {_ble.Address} reconnected");

                    SetConnectionState(ConnectionState.Connected);
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
                    delaySeconds = Math.Min(10, delaySeconds + 1);
                }
            }
        }
        internal static MeDevice Create(BtlePeripheral ble)
        {
            return new MeDevice(ble);
        }

        public async Task ConnectAsync()
        {
            // Bluez sometimes aborts a connect that races with the scan teardown
            // ("le-connection-abort-by-local"). Retry a few times before giving up,
            // and only enable the auto-reconnect path once the full session is set
            // up — otherwise a mid-init failure spawns a background reconnect loop
            // while the caller is still trying to handle the original error.
            const int maxAttempts = 3;
            Exception? lastError = null;
            for (int attempt = 1; attempt <= maxAttempts; attempt++)
            {
                try
                {
                    if (!await _ble.IsConnectedAsync().ConfigureAwait(false))
                    {
                        await _ble.ConnectAsync().ConfigureAwait(false);
                    }
                    await InitializeSessionAsync();

                    _shouldTryReconnect = true;
                    SetConnectionState(ConnectionState.Connected);
                    return;
                }
                catch (Exception ex)
                {
                    lastError = ex;
                    Console.WriteLine($"Connect attempt {attempt}/{maxAttempts} failed: {ex.Message}");

                    try { await _ble.DisconnectAsync().ConfigureAwait(false); }
                    catch { /* already gone */ }

                    if (attempt < maxAttempts)
                    {
                        await Task.Delay(TimeSpan.FromMilliseconds(500 * attempt)).ConfigureAwait(false);
                    }
                }
            }

            throw lastError ?? new InvalidOperationException("Failed to connect to ME device.");
        }

        private async Task InitializeSessionAsync()
        {
            // Retrieve the list of services to ensure the device is fully initialized before proceeding
            await _ble.GetServicesAsync();

            // On a reconnect, the lib's _callbacks dict still holds the previous
            // entry, so calling Register again only stacks delegates and skips the
            // native PeripheralSubscribe. Force a clean unregister/re-register so
            // the CCCD is re-armed on the peer.
            if (_notificationsRegistered)
            {
                try
                {
                    await _ble.UnregisterNotificationCallback(
                        RaceChronoIds.ServiceUuid,
                        RaceChronoIds.CanBusCharacteristicUuid,
                        DataReceived);
                }
                catch (Exception ex)
                {
                    Console.WriteLine($"Unregister-on-reconnect failed (continuing): {ex.Message}");
                }
            }

            await _ble.RegisterNotificationCallback(
                RaceChronoIds.ServiceUuid,
                RaceChronoIds.CanBusCharacteristicUuid,
                DataReceived);
            _notificationsRegistered = true;

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
                if (data.Length < 2)
                {
                    return;
                }

                ushort canId = BitConverter.ToUInt16(data);
                ReadOnlySpan<byte> dataPacket = data[4..];

                Pid pid = (Pid)(canId & 0x0FFF);
                if (!Enum.IsDefined(pid))
                {
                    return;
                }

                ICanFrame frame = CanDecoder.Decode(pid, dataPacket);
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
