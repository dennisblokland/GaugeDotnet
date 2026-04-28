using System;
using System.Buffers.Binary;
using System.Collections.Generic;
using System.Globalization;
using System.Runtime.CompilerServices;
using System.Threading.Tasks;
using ME1_4NET;
using ME1_4NET.Frames;
using VaettirNet.Btleplug;

namespace GaugeDotnet
{
    /// class created from <see cref="CreateAsync"/>. Dice can be found and connected using
    /// <see cref="StartScan"/>
    /// </summary>
    public sealed class BleManager : IDisposable
    {
        private readonly BtleManager _ble;

        private BleManager(BtleManager ble)
        {
            _ble = ble;
        }

        public static Task<BleManager> CreateAsync()
        {
            BtleManager manager = BtleManager.Create();
            return Task.FromResult(new BleManager(manager));
        }

        /// <summary>
        /// Scan for pixels devices. When a die is found, it will be returned as part of the enumerable. The scan will
        /// continue until the cancellationToken is cancelled
        /// </summary>
        /// <param name="findAll">True to find and return all devices, false to only find devices in savedIdentifiers list</param>
        /// <param name="savedIdentifiers">List of devices to return and connect be default (even if findAll is false)</param>
        /// <param name="cancellationToken">CancellationToken to stop scanning</param>
        /// <returns>Enumerable of all devices found.</returns>
        public async Task<MeDevice?> ScanAsync(
            bool findAll,
            IEnumerable<string> savedIdentifiers = null,
            CancellationToken cancellationToken = default)
        {
            if (!findAll && savedIdentifiers == null)
                throw new ArgumentException($"One of {nameof(findAll)} or {nameof(savedIdentifiers)} must be set");

            HashSet<ulong> idSet = savedIdentifiers?.Select(s => ulong.Parse(s, NumberStyles.AllowHexSpecifier)).ToHashSet();

            await foreach (BtlePeripheral peripheral in _ble.GetPeripherals([RaceChronoIds.ServiceUuid], false, cancellationToken))
            {
                if (!findAll && !idSet.Contains(peripheral.Address))
                {
                    peripheral.Dispose();
                    continue;
                }

                return MeDevice.Create(peripheral);
            }

            return null;
        }

        public async Task<MeDevice?> ReattachAsync(
            IEnumerable<string> savedIdentifiers,
            TimeSpan? timeout = null,
            CancellationToken cancellationToken = default
        )
        {
            timeout ??= TimeSpan.FromSeconds(30);
            List<string> items = savedIdentifiers.ToList();
            var src = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            src.CancelAfter(timeout.Value);

            MeDevice? me = await ScanAsync(false, items, src.Token);
            if (me != null)
            {
                await me.ConnectAsync();
                return me;
            }
            return me;

        }

        public void Dispose()
        {
            _ble.Dispose();
        }
    }

}