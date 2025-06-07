namespace GaugeDotnet
{
    using RG35XX.Core.GamePads;
    using System;
    using System.Buffers.Binary;
    using System.Collections.Generic;
    using System.Linq;
    using System.Text;
    using System.Threading.Tasks;
    using VaettirNet.Btleplug;
    using VaettirNet.Btleplug.Interop;

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

        public BLE()
        {

        }

        public async Task Start()
        {
            BtleManager.SetLogLevel(BtleLogLevel.Debug);
            var manager = BtleManager.Create();
            List<BtlePeripheral> all = [];
            CancellationTokenSource src = new();
            src.CancelAfter(TimeSpan.FromSeconds(60));
            try
            {
                ScanPeripherals(manager, src, all);
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
        private void ScanPeripherals(BtleManager manager, CancellationTokenSource src, List<BtlePeripheral> all)
        {

            var _keyThread = new Thread(async () =>
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
                                    ushort interval = 50;


                                    p.Write(service.Uuid, c.Uuid, MagicAllPidPackage, false).Wait();

                                }
                                if (c.Uuid == CanBusCharacteristicUuid)
                                {
                                    await p.RegisterNotificationCallback(service.Uuid, c.Uuid, NotifyFound);
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
            ushort canId = BinaryPrimitives.ReadUInt16BigEndian(data);
            ReadOnlySpan<byte> dataPacket = data.Slice(2);

            if (canId == 3)
            {
                var result = Me1_4Me1_4Parser.Unpack(dataPacket);
                if (result == null)
                {
                    Console.WriteLine("Failed to unpack Me1_4Me1_4 data.");
                    return;
                }

                return;

            }
            
            Console.WriteLine($"Received CAN ID: {canId:X4} from {peripheral.GetId()}");
           
        }
    }
}