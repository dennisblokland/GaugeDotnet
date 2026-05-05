using ME1_4NET;

namespace GaugeDotnet.Devices
{
    public interface IMeDevice
    {
        MEData Data { get; }
        bool IsConnected { get; }
        ConnectionState ConnectionState { get; }
        event Action<IMeDevice, ConnectionState> ConnectionStateChanged;
        Task ConnectAsync();
    }
}
