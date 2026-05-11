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

		public GameLoop(AppConfig appConfig, IMeDevice? meDevice, GaugeSDL gaugeSDL, int screenWidth, int screenHeight)
		{
			_appConfig = appConfig;
			_meDevice = meDevice;
			_screenWidth = screenWidth;
			_screenHeight = screenHeight;

			_gaugeSDL = gaugeSDL;
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

				HandleReset(key, sdlKey);
				HandleScreenNavigation(key);
				UpdateGaugeData();
				Render();
			}

			RenderExitMessage();
		}

		private void HandleReset(GamepadKey key, SDL_Keycode? sdlKey)
		{
			if (key == GamepadKey.R1_DOWN || sdlKey == SDL_Keycode.SDLK_r)
			{
				if (_screens.Count > 0 && _currentScreen < _screens.Count)
				{
					_screens[_currentScreen].Gauge.ResetSavedState();
				}
			}
		}

		private void RenderExitMessage()
		{
			var accent = new SKColor(0xFF, 0x6B, 0x35); // orange
			float cx = _screenWidth / 2f;
			float cy = _screenHeight / 2f;

			using SKPaint titlePaint = new()
			{
				IsAntialias = true,

			};
			using SKTypeface titleTypeface = FontHelper.Default;

			using SKFont titleFont = new(titleTypeface, 64);

			using SKPaint subPaint = new()
			{
				Color = new SKColor(180, 180, 180),
				IsAntialias = true,
			};

			using SKFont subFont = new(titleTypeface, 24);

			using SKPaint linePaint = new()
			{
				Color = accent,
				StrokeWidth = 2,
				IsAntialias = true,
				Style = SKPaintStyle.Stroke,
			};

			const int steps = 20;
			for (int i = 0; i <= steps; i++)
			{
				float t = i / (float)steps;
				byte alpha = (byte)(255 * t);

				SKCanvas canvas = _gaugeSDL.GetCanvas();
				canvas.Clear(SKColors.Black);

				float lineY1 = cy - 52;
				float lineY2 = cy + 30;
				float halfLen = _screenWidth * 0.38f * t;

				linePaint.Color = accent.WithAlpha(alpha);
				canvas.DrawLine(cx - halfLen, lineY1, cx + halfLen, lineY1, linePaint);
				canvas.DrawLine(cx - halfLen, lineY2, cx + halfLen, lineY2, linePaint);

				titlePaint.Color = SKColors.White.WithAlpha(alpha);

				canvas.DrawText("EXITING", cx, cy + 10, SKTextAlign.Center, titleFont, titlePaint);

				subPaint.Color = new SKColor(180, 180, 180, alpha);
				canvas.DrawText("Goodbye", cx, cy + 58, SKTextAlign.Center,  subFont, subPaint);

				_gaugeSDL.FlushAndSwap();
				Thread.Sleep(18);
			}

			Thread.Sleep(600);
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

		public void Dispose()
		{
			_input.Dispose();
			_gaugeSDL.Dispose();
		}

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
