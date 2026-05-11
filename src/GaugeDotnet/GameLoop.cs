using System.Diagnostics;
using GaugeDotnet.Configuration;
using GaugeDotnet.Devices;
using GaugeDotnet.Gauges;
using GaugeDotnet.Rendering;
using RG35XX.Libraries;
using SkiaSharp;
using static SDL2.SDL;
using RG35XX.Core.GamePads;

namespace GaugeDotnet
{
	public class GameLoop : IDisposable
	{
		private readonly AppConfig _appConfig;
		private readonly IMeDevice? _meDevice;
		private readonly int _screenWidth;
		private readonly int _screenHeight;

		private readonly GaugeSDL _gaugeSDL;
		private readonly InputHandler _input;
		private readonly FpsCounter _fps;
		private readonly Stopwatch _stopwatch;

		private List<(BaseGauge Gauge, string DataSource)> _screens;
		private int _currentScreen;
		private ConfigEditor? _configEditor;
		private double _lastUpdate;
		private double _lastKeepAlive;

		public GameLoop(AppConfig appConfig, IMeDevice? meDevice, int screenWidth, int screenHeight)
		{
			_appConfig = appConfig;
			_meDevice = meDevice;
			_screenWidth = screenWidth;
			_screenHeight = screenHeight;

			_gaugeSDL = new GaugeSDL(screenWidth: screenWidth, screenHeight: screenHeight);
			_input = new InputHandler();
			_fps = new FpsCounter();
			_stopwatch = Stopwatch.StartNew();

			_screens = GaugeFactory.BuildScreens(appConfig, screenWidth, screenHeight);
		}

		public void Run()
		{
			bool running = true;

			while (running)
			{
				_input.Poll();

				if (_input.QuitRequested)
				{
					break;
				}

				GamepadKey key = _input.LastGamepadKey;
				SDL_Keycode? sdlKey = _input.LastSdlKey;

				if (HandleConfigEditor(key, sdlKey))
				{
					continue;
				}

				if (key == GamepadKey.MENU_DOWN)
				{
					break;
				}

				if (key == GamepadKey.SELECT_DOWN || sdlKey == SDL_Keycode.SDLK_TAB)
				{
					_configEditor = new ConfigEditor(_appConfig, _screenWidth, _screenHeight);
					continue;
				}

				HandleScreenNavigation(key);
				UpdateGaugeData();
				Render();
			}
		}

		private bool HandleConfigEditor(GamepadKey key, SDL_Keycode? sdlKey)
		{
			if (_configEditor == null)
			{
				return false;
			}

			_configEditor.HandleInput(key, sdlKey);

			if (!_configEditor.IsActive)
			{
				if (_configEditor.ConfigChanged)
				{
					_screens = GaugeFactory.BuildScreens(_appConfig, _screenWidth, _screenHeight);
					if (_currentScreen >= _screens.Count)
					{
						_currentScreen = Math.Max(0, _screens.Count - 1);
					}
				}
				_configEditor = null;
				return false;
			}

			SKCanvas canvas = _gaugeSDL.GetCanvas();
			_configEditor.Draw(canvas);
			_gaugeSDL.FlushAndSwap();
			return true;
		}

		private void HandleScreenNavigation(GamepadKey key)
		{
			if (key == GamepadKey.RIGHT && _screens.Count > 1)
			{
				_currentScreen = (_currentScreen + 1) % _screens.Count;
			}
			else if (key == GamepadKey.LEFT && _screens.Count > 1)
			{
				_currentScreen = (_currentScreen - 1 + _screens.Count) % _screens.Count;
			}
		}

		private void UpdateGaugeData()
		{
			double now = _stopwatch.Elapsed.TotalSeconds;

			if (now - _lastKeepAlive >= 3.0)
			{
				_lastKeepAlive = now;
				ScreenKeepAlive.Poke();
			}

			if (now - _lastUpdate < 0.05)
			{
				return;
			}

			if (_meDevice != null && _meDevice.IsConnected && _screens.Count > 0)
			{
				(BaseGauge g, string dataSource) = _screens[_currentScreen];
				GaugeFactory.UpdateGaugeValues(g, dataSource, _meDevice);
			}

			_lastUpdate = now;
		}

		public void Dispose() => _gaugeSDL.Dispose();

		private void Render()
		{
			SKCanvas canvas = _gaugeSDL.GetCanvas();
			canvas.Clear(SKColors.Black);

			if (_screens.Count > 0 && _currentScreen < _screens.Count)
			{
				_screens[_currentScreen].Gauge.Draw(canvas);
			}

			_fps.Tick();
			_fps.Draw(canvas);

			_gaugeSDL.FlushAndSwap();
		}
	}
}
