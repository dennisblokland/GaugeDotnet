using System.Diagnostics;
using SkiaSharp;

namespace GaugeDotnet.Rendering
{
	public class FpsCounter
	{
		private readonly Stopwatch _stopwatch = Stopwatch.StartNew();
		private int _frameCount;
		private double _lastReport;
		private double _currentFps;
		private string _fpsText = "FPS: 0.00";

		private readonly SKPaint _paint = new()
		{
			Color = SKColors.White,
			IsAntialias = true
		};

		private readonly SKFont _font = new(FontHelper.Default) { Size = 20 };

		public void Tick()
		{
			_frameCount++;
			double elapsed = _stopwatch.Elapsed.TotalSeconds;
			if (elapsed - _lastReport >= 1.0)
			{
				_currentFps = _frameCount / (elapsed - _lastReport);
				_lastReport = elapsed;
				_frameCount = 0;
				_fpsText = $"FPS: {_currentFps:F2}";
			}
		}

		public void Draw(SKCanvas canvas)
		{
			canvas.DrawText(_fpsText, 10, 25, _font, _paint);
		}

		public void Dispose()
		{
			_paint.Dispose();
			_font.Dispose();
		}
	}
}
