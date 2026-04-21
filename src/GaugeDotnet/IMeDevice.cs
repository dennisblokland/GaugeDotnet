namespace GaugeDotnet
{
    public interface IMeDevice
    {
        decimal afr { get; }
        bool IsConnected { get; }
        ConnectionState ConnectionState { get; }
        event Action<IMeDevice, ConnectionState> ConnectionStateChanged;
        Task ConnectAsync();
    }
}
