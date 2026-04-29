using static SDL2.SDL;
using RG35XX.Core.GamePads;
using RG35XX.Core.Interfaces;
using RG35XX.Libraries;

namespace GaugeDotnet
{
	public class InputHandler
	{
		private readonly IGamePadReader _gamePadReader;

		public SDL_Keycode? LastSdlKey { get; private set; }
		public GamepadKey LastGamepadKey { get; private set; }
		public bool QuitRequested { get; private set; }

		public InputHandler()
		{
			_gamePadReader = new GamePadReader();
			_gamePadReader.Initialize();
		}

		public void Poll()
		{
			LastSdlKey = null;
			QuitRequested = false;

			while (SDL_PollEvent(out SDL_Event e) == 1)
			{
				switch (e.type)
				{
					case SDL_EventType.SDL_QUIT:
						QuitRequested = true;
						break;
					case SDL_EventType.SDL_KEYDOWN:
					{
						SDL_KeyboardEvent keyEvent = e.key;
						SDL_Keycode keycode = keyEvent.keysym.sym;
						byte repeat = keyEvent.repeat;
						if (repeat == 0)
						{
							LastSdlKey = keycode;
						}
						KeyBus.OnKeyDown(keycode);
						break;
					}
					case SDL_EventType.SDL_KEYUP:
					{
						SDL_KeyboardEvent keyEvent = e.key;
						SDL_Keycode keycode = keyEvent.keysym.sym;
						KeyBus.OnKeyUp(keycode);
						break;
					}
				}
			}

			LastGamepadKey = _gamePadReader.ReadInput();
		}
	}
}
