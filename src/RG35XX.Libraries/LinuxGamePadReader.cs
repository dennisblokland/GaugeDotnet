using RG35XX.Core.GamePads;
using RG35XX.Core.Interfaces;
using System.Collections.Concurrent;
using System.Runtime.InteropServices;

namespace RG35XX.Libraries
{
    public partial class LinuxGamePadReader : IGamePadReader
    {
        private readonly byte[] _eventBuffer = new byte[8];
        private readonly ConcurrentQueue<GamepadKey> _keyBuffer = [];
        private readonly CancellationTokenSource _cts = new();

        private Thread? _keyThread;
        private FileStream? _stream;
        private volatile bool _disposed;

        public void ClearBuffer()
        {
            _keyBuffer.Clear();
        }

        public void Initialize(string devicePath = "/dev/input/js0")
        {
            if (_disposed)
            {
                throw new ObjectDisposedException(nameof(LinuxGamePadReader));
            }

            if (_stream != null)
            {
                // Already initialized; ignore re-init to keep call sites idempotent.
                return;
            }

            try
            {
                _stream = new FileStream(devicePath, FileMode.Open, FileAccess.Read);
            }
            catch (Exception e)
            {
                Console.WriteLine($"Failed to open gamepad device: {e.Message}");
                return;
            }

            _keyThread = new Thread(ReadLoop)
            {
                Priority = ThreadPriority.AboveNormal,
                IsBackground = true,
                Name = "LinuxGamePadReader"
            };
            _keyThread.Start();
        }

        public GamepadKey ReadInput()
        {
            if (_keyBuffer.TryDequeue(out GamepadKey key))
            {
                return key;
            }

            return GamepadKey.None;
        }

        public void Dispose()
        {
            if (_disposed)
            {
                return;
            }
            _disposed = true;

            _cts.Cancel();

            // Disposing the stream unblocks the reader thread's blocking Read().
            try
            {
                _stream?.Dispose();
            }
            catch
            {
                // ignore
            }

            try
            {
                _keyThread?.Join(TimeSpan.FromMilliseconds(500));
            }
            catch
            {
                // ignore
            }

            _cts.Dispose();
            _stream = null;
            _keyThread = null;
        }

        private void ReadLoop()
        {
            CancellationToken token = _cts.Token;
            while (!token.IsCancellationRequested)
            {
                GamepadKey key = ReadFromStream();

                if (token.IsCancellationRequested)
                {
                    break;
                }

                if (key != GamepadKey.None)
                {
                    _keyBuffer.Enqueue(key);
                }
            }
        }

        private GamepadKey ReadFromStream()
        {
            FileStream? stream = _stream;
            if (stream == null)
            {
                return GamepadKey.None;
            }

            try
            {
                int read = stream.Read(_eventBuffer, 0, 8);

                if (read != 8)
                {
                    return GamepadKey.None;
                }

                JoystickEvent evt = new();
                GCHandle handle = GCHandle.Alloc(_eventBuffer, GCHandleType.Pinned);
                try
                {
                    object? obj = Marshal.PtrToStructure(
                        handle.AddrOfPinnedObject(),
                        typeof(JoystickEvent));
                    if (obj != null)
                    {
                        evt = (JoystickEvent)obj;
                    }
                    else
                    {
                        return GamepadKey.None;
                    }
                }
                finally
                {
                    handle.Free();
                }

                JoystickInput input = new()
                {
                    Type = evt.Type,
                    Number = evt.Number,
                    Value = (ushort)evt.Value
                };

                int value = input.Number;

                value <<= 16;

                value |= input.Value;

                GamepadKey gcKey = (GamepadKey)value;

                return gcKey;
            }
            catch
            {
                return GamepadKey.None;
            }
        }
    }
}
