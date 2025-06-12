namespace GaugeDotnet
{
    using System;
    using System.Buffers.Binary;
    using System.Collections.Generic;
    using System.Threading.Tasks;
    using ME1_4NET;
    using ME1_4NET.Frames;
    using ME1_4NET;
    using VaettirNet.Btleplug;

    public class BLE
    {
        static readonly Guid ServiceUuid =
        new Guid("0000FFF0-0000-1000-8000-00805F9B34FB");

        private static readonly Guid PidCharacteristicUuid =
         new Guid("00000002-0000-1000-8000-00805F9B34FB");

        private static readonly Guid CanBusCharacteristicUuid =
            new Guid("00000001-0000-1000-8000-00805F9B34FB");

        private static readonly byte[] MagicAllPidPackage =
                            [
                                0x01,
                                (50 >> 8),
                                (50 & 0xFF)
                            ];

        public bool IsRunning { get; private set; } = false;
        public MEData MeData { get; }

        private readonly List<BtlePeripheral.PeripheralNotifyDataReceivedCallback>
                _notificationCallbacks = [];

        private readonly List<BtlePeripheral> all = [];

        public BLE(MEData meData)
        {
            MeData = meData;

        }

        public async Task Start()
        {
            BtleManager.SetLogLevel(BtleLogLevel.Error);
            BtleManager manager = BtleManager.Create();
            all.Clear();
            CancellationTokenSource src = new();
            src.CancelAfter(TimeSpan.FromSeconds(60));
            try
            {
                ScanPeripherals(manager, src);
            }
            catch (OperationCanceledException)
            {
                Console.WriteLine("Scan cancelled or timed out.");
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Error during scan: {ex.Message}");
            }

            Console.WriteLine("Shutting down");

            foreach (BtlePeripheral p in all)
            {
                await p.DisconnectAsync();
                p.Dispose();
            }
        }
        private void ScanPeripherals(BtleManager manager, CancellationTokenSource src)
        {

            Thread _keyThread = new Thread(async () =>
            {
                do
                {
                    await foreach (BtlePeripheral p in manager.GetPeripherals([ServiceUuid], includeServices: false, src.Token))
                    {
                        all.Add(p);
                        Console.WriteLine($"Found device: {p.GetId()}");
                        await p.ConnectAsync();
                        bool isConnected = await p.IsConnectedAsync();
                        Console.WriteLine($"Connected ({isConnected})");
                        foreach (BtleService service in await p.GetServicesAsync())
                        {
                            Console.WriteLine($"  S: {service.Uuid}");
                            foreach (BtleCharacteristic c in service.Characteristics)
                            {
                                Console.WriteLine($"Characteristic: {c.Uuid}");
                                if (c.Uuid == PidCharacteristicUuid)
                                {
                                    await p.Write(service.Uuid, c.Uuid, MagicAllPidPackage, false);

                                }
                                if (c.Uuid == CanBusCharacteristicUuid)
                                {
                                    BtlePeripheral.PeripheralNotifyDataReceivedCallback callback = new(NotifyFound);
                                    _notificationCallbacks.Add(callback);

                                    await p.RegisterNotificationCallback(service.Uuid, c.Uuid, callback);
                                }
                            }
                        }
                    }
                } while (true);
            })
            {
                Priority = ThreadPriority.AboveNormal
            };

            _keyThread.Start();

        }

        private void NotifyFound(BtlePeripheral peripheral, Guid service, Guid characteristic, Span<byte> data)
        {
            IsRunning = true;
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
            if (frame is ME1_1 me1_1)
            {
                // Console.WriteLine($"RPM: {me1_1.Rpm}");
                // Console.WriteLine($"Throttle: {me1_1.ThrottlePosition}");
                // Console.WriteLine($"MAP: {me1_1.Map}");
                // Console.WriteLine($"IAT: {me1_1.Iat}");
            }
            if (frame is ME1_2 me1_2)
            {
                //afr
                Console.WriteLine($"AFR: {me1_2.AfrCurr1}");
                MeData.AfrCurr1 = me1_2.AfrCurr1;

            }

            // else if (frame is ME1_6 me1_6)
            // // {
            // //     Console.WriteLine($"Gear pos: {me1_6.GearPos}");
            // //     Console.WriteLine($"Map target: {me1_6.MapTarget}");
            // //     Console.WriteLine($"Vehicle speed: {me1_6.VehicleSpeed}");
            // //     Console.WriteLine($"EPS EV Mask: {me1_6.EpsEvMsk}");
            // }




        }

        public void Stop()
        {
            IsRunning = false;
            foreach (BtlePeripheral p in all)
            {
                p.Dispose();
            }
            all.Clear();
            _notificationCallbacks.Clear();
        }


    }
}