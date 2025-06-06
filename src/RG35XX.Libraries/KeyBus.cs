using RG35XX.Core.GamePads;
using SDL2;
using System.Collections.Concurrent;

namespace RG35XX.Libraries
{
    public static class KeyBus
    {
        // Replace BlockingCollection with ConcurrentQueue for non-blocking behavior
        private static readonly ConcurrentQueue<GamepadKey> _keys = new();

        public static void ClearBuffer()
        {
            while (_keys.TryDequeue(out _)) { }
        }

        public static void OnKeyDown(SDL.SDL_Keycode keyCode)
        {
            GamepadKey key = GamepadKey.None;

            switch (keyCode)
            {
                case SDL.SDL_Keycode.SDLK_a:
                    key = GamepadKey.A_DOWN;
                    break;
                case SDL.SDL_Keycode.SDLK_RETURN:
                    key = GamepadKey.START_DOWN;
                    break;
                case SDL.SDL_Keycode.SDLK_b:
                    key = GamepadKey.B_DOWN;
                    break;
                case SDL.SDL_Keycode.SDLK_y:
                    key = GamepadKey.Y_DOWN;
                    break;
                case SDL.SDL_Keycode.SDLK_x:
                    key = GamepadKey.X_DOWN;
                    break;
                case SDL.SDL_Keycode.SDLK_UP:
                    key = GamepadKey.UP;
                    break;
                case SDL.SDL_Keycode.SDLK_DOWN:
                    key = GamepadKey.DOWN;
                    break;
                case SDL.SDL_Keycode.SDLK_LEFT:
                    key = GamepadKey.LEFT;
                    break;
                case SDL.SDL_Keycode.SDLK_RIGHT:
                    key = GamepadKey.RIGHT;
                    break;
                case SDL.SDL_Keycode.SDLK_ESCAPE:
                    key = GamepadKey.MENU_DOWN;
                    break;
                case SDL.SDL_Keycode.SDLK_COMMA:
                    key = GamepadKey.L1_DOWN;
                    break;
                case SDL.SDL_Keycode.SDLK_PERIOD:
                    key = GamepadKey.R1_DOWN;
                    break;
            }

            if (key != GamepadKey.None)
            {
                _keys.Enqueue(key);
            }
        }

        public static void OnKeyUp(SDL.SDL_Keycode keyCode)
        {
            GamepadKey key = GamepadKey.None;

            switch (keyCode)
            {
                case SDL.SDL_Keycode.SDLK_a:
                    key = GamepadKey.A_UP;
                    break;
                case SDL.SDL_Keycode.SDLK_RETURN:
                    key = GamepadKey.START_UP;
                    break;
                case SDL.SDL_Keycode.SDLK_b:
                    key = GamepadKey.B_UP;
                    break;
                case SDL.SDL_Keycode.SDLK_y:
                    key = GamepadKey.Y_UP;
                    break;
                case SDL.SDL_Keycode.SDLK_x:
                    key = GamepadKey.X_UP;
                    break;
                case SDL.SDL_Keycode.SDLK_UP:
                    key = GamepadKey.UP_DOWN_UP;
                    break;
                case SDL.SDL_Keycode.SDLK_DOWN:
                    key = GamepadKey.UP_DOWN_UP;
                    break;
                case SDL.SDL_Keycode.SDLK_LEFT:
                    key = GamepadKey.LEFT_RIGHT_UP;
                    break;
                case SDL.SDL_Keycode.SDLK_RIGHT:
                    key = GamepadKey.LEFT_RIGHT_UP;
                    break;
                case SDL.SDL_Keycode.SDLK_ESCAPE:
                    key = GamepadKey.MENU_UP;
                    break;
                case SDL.SDL_Keycode.SDLK_COMMA:
                    key = GamepadKey.L1_UP;
                    break;
                case SDL.SDL_Keycode.SDLK_PERIOD:
                    key = GamepadKey.R1_UP;
                    break;
            }

            if (key != GamepadKey.None)
            {
                _keys.Enqueue(key);
            }
        }

        public static GamepadKey ReadInput()
        {
            // Non-blocking: Try to dequeue, return null if none available
            if (_keys.TryDequeue(out GamepadKey key))
                return key;
            return GamepadKey.None;
        }
    }
}