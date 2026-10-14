using RG35XX.Core.GamePads;
using RG35XX.Core.Interfaces;

namespace RG35XX.Libraries
{
    public class GamePadReader : IGamePadReader
    {
        private readonly IGamePadReader _gamePadReader;

        public GamePadReader()
        {
#if DEBUG
            _gamePadReader = new RG35XX.Libraries.KeyboardInput();

#else
            _gamePadReader = new RG35XX.Libraries.LinuxGamePadReader();
#endif
        }

        /// <summary>
        /// True when no joystick device is read, so the keyboard stands in for the
        /// gamepad (DEBUG builds, or a desktop without /dev/input/js0).
        /// </summary>
        public bool UsesKeyboard => _gamePadReader is not LinuxGamePadReader { IsOpen: true };

        public void ClearBuffer()
        {
            _gamePadReader.ClearBuffer();
        }

        public void Initialize(string devicePath = "/dev/input/js0")
        {
            _gamePadReader.Initialize(devicePath);
        }

        public GamepadKey ReadInput()
        {
            return _gamePadReader.ReadInput();
        }

        public void Dispose()
        {
            _gamePadReader.Dispose();
        }
    }
}
