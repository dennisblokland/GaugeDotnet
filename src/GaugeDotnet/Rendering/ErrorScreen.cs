using RG35XX.Core.GamePads;
using RG35XX.Core.Interfaces;
using RG35XX.Libraries;
using SkiaSharp;
using static SDL2.SDL;

namespace GaugeDotnet.Rendering
{
	public static class ErrorScreen
	{
		public static void Show(string message)
		{
			int w = 640, h = 480;
			GaugeSDL sdl = new(screenWidth: w, screenHeight: h);
			IGamePadReader gp = new GamePadReader();
			gp.Initialize();

			using var bitmap = new SKBitmap(w, h);
			using var bmpCanvas = new SKCanvas(bitmap);
			bmpCanvas.Clear(SKColors.Black);

			using SKPaint paint = new() { Color = SKColors.Red, IsAntialias = true };
			SKTypeface typeface = FontHelper.GetFont("Race Sport");
			using SKFont font = new(typeface, 24);

			float y = 200;
			foreach (string line in message.Split('\n'))
			{
				float lw = font.MeasureText(line);
				bmpCanvas.DrawText(line, (w - lw) / 2, y, font, paint);
				y += 34;
			}

			paint.Color = SKColors.White;
			const string exitMsg = "Press any button to exit";
			float ew = font.MeasureText(exitMsg);
			bmpCanvas.DrawText(exitMsg, (w - ew) / 2, y + 20, font, paint);

			SKCanvas canvas = sdl.GetCanvas();
			canvas.Clear(SKColors.Black);
			using var image = SKImage.FromBitmap(bitmap);
			canvas.DrawImage(image, 0, 0);
			sdl.FlushAndSwap();

			while (true)
			{
				while (SDL_PollEvent(out SDL_Event e) == 1)
				{
					if (e.type == SDL_EventType.SDL_QUIT || e.type == SDL_EventType.SDL_KEYDOWN)
					{
						SDL_Quit();
						return;
					}
				}
				GamepadKey key = gp.ReadInput();
				if (key != GamepadKey.None)
				{
					SDL_Quit();
					return;
				}
				Thread.Sleep(50);
			}
		}
	}
}
