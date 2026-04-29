using GaugeDotnet.Configuration;
using GaugeDotnet.Gauges.Models;
using SkiaSharp;

namespace GaugeDotnet.Gauges
{
	public class GridGauge : BaseGauge
	{
		private const int Columns = 2;
		private const int Rows = 2;

		private static readonly SKColor ValueColor = SKColors.White;
		private static readonly SKColor LabelColor = new(120, 120, 120);
		private static readonly SKColor LineColor = new(60, 60, 60);

		private readonly int _screenWidth;
		private readonly int _screenHeight;
		private readonly List<GridCellConfig> _cells;
		private readonly float[] _cellValues;
		private readonly SKTypeface _raceFont;

		private readonly SKBitmap _staticBitmap;
		private readonly SKCanvas _staticCanvas;
		private readonly SKPaint _valuePaint;
		private readonly SKFont _valueFont;

		public GridGauge(
			GridGaugeSettings settings,
			int screenWidth = 640,
			int screenHeight = 480
		) : base(settings)
		{
			_screenWidth = screenWidth;
			_screenHeight = screenHeight;
			_cells = settings.Cells;
			_cellValues = new float[_cells.Count];
			_raceFont = FontHelper.GetFont("Race Sport");

			_staticBitmap = new SKBitmap(screenWidth, screenHeight, SKColorType.Rgba8888, SKAlphaType.Premul);
			_staticCanvas = new SKCanvas(_staticBitmap);

			float cellHeight = screenHeight / (float)Rows;
			float valueSize = cellHeight * 0.35f;
			_valuePaint = new SKPaint
			{
				Color = ValueColor,
				IsAntialias = true,
			};
			_valueFont = new SKFont(_raceFont, valueSize);
		}

		public int CellCount => _cells.Count;

		public GridCellConfig GetCellConfig(int index) => _cells[index];

		public void SetCellValue(int index, float value)
		{
			if (index >= 0 && index < _cellValues.Length)
			{
				_cellValues[index] = value;
			}
		}

		public override void Draw(SKCanvas canvas)
		{
			if (!StaticCacheValid)
			{
				UpdateStaticBackground();
				StaticCacheValid = true;
			}

			canvas.DrawBitmap(_staticBitmap, 0, 0);

			DrawValues(canvas);
		}

		private void UpdateStaticBackground()
		{
			DrawBackground(_staticCanvas, _screenWidth, _screenHeight);

			float cellWidth = _screenWidth / (float)Columns;
			float cellHeight = _screenHeight / (float)Rows;

			using SKPaint linePaint = new()
			{
				Style = SKPaintStyle.Stroke,
				StrokeWidth = 1f,
				Color = LineColor,
				IsAntialias = true,
			};

			// Draw grid lines
			for (int col = 1; col < Columns; col++)
			{
				float x = col * cellWidth;
				_staticCanvas.DrawLine(x, 0, x, _screenHeight, linePaint);
			}
			for (int row = 1; row < Rows; row++)
			{
				float y = row * cellHeight;
				_staticCanvas.DrawLine(0, y, _screenWidth, y, linePaint);
			}

			// Draw title and unit labels in each cell
			using SKPaint labelPaint = new()
			{
				Color = LabelColor,
				IsAntialias = true,
			};

			float titleSize = cellHeight * 0.16f;
			float unitSize = cellHeight * 0.14f;

			for (int i = 0; i < _cells.Count && i < Columns * Rows; i++)
			{
				int col = i % Columns;
				int row = i / Columns;
				float cx = col * cellWidth + cellWidth / 2f;
				float cellTop = row * cellHeight;

				using SKFont titleFont = new(_raceFont, titleSize);
				_staticCanvas.DrawText(_cells[i].Title, cx, cellTop + titleSize + 4f, SKTextAlign.Center, titleFont, labelPaint);

				using SKFont unitFont = new(_raceFont, unitSize);
				_staticCanvas.DrawText(_cells[i].Unit, cx, cellTop + cellHeight - 6f, SKTextAlign.Center, unitFont, labelPaint);
			}
		}

		private void DrawValues(SKCanvas canvas)
		{
			float cellWidth = _screenWidth / (float)Columns;
			float cellHeight = _screenHeight / (float)Rows;
			float valueSize = _valueFont.Size;

			for (int i = 0; i < _cells.Count && i < Columns * Rows; i++)
			{
				int col = i % Columns;
				int row = i / Columns;
				float cx = col * cellWidth + cellWidth / 2f;
				float cy = row * cellHeight + cellHeight / 2f + valueSize * 0.15f;

				string text = _cellValues[i].ToString($"F{_cells[i].Decimals}");

				canvas.DrawText(text, cx, cy, SKTextAlign.Center, _valueFont, _valuePaint);
			}
		}
	}
}
