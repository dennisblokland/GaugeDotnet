using System;

namespace GaugeDotnet
{
    internal static class RaceChronoIds
    {
        private const string ServiceId = "0000FFF0-0000-1000-8000-00805F9B34FB";
        private const string PidCharacteristicId = "00000002-0000-1000-8000-00805F9B34FB";
        private const string CanBusCharacteristicId = "00000001-0000-1000-8000-00805F9B34FB";
        internal static readonly Guid ServiceUuid = Guid.Parse(ServiceId);
        internal static readonly Guid PidCharacteristicUuid = Guid.Parse(PidCharacteristicId);
        internal static readonly Guid CanBusCharacteristicUuid = Guid.Parse(CanBusCharacteristicId);
    }
}