using GaugeDotnet.Gauges.Models;
using SkiaSharp;

namespace GaugeDotnet.Gauges
{
    public abstract class BaseGauge
    {
        private float _value;
        private readonly float _minValue;
        private readonly float _maxValue;
        private string _colorHex = "#00FFFF";          // active color
        private SKColor _activeColor = SKColor.Parse("#00FFFF");
        private SKColor _inactiveColor = SKColor.Parse("#001919");
        private readonly string _unit;
        private readonly string _title;

        public bool StaticCacheValid { get; protected set; } = false;

        protected BaseGauge(BaseGaugeSettings settings)
        {
            _value = settings.InitialValue;
            _minValue = settings.MinValue;
            _maxValue = settings.MaxValue;
            _unit = settings.Unit ?? "";
            _title = settings.Title ?? "";
            ComputeInactiveColor();
        }

        public abstract void Draw(SKCanvas canvas);

        public virtual void SetValue(float v)
        {
            _value = v;
        }

        public float Value => _value;
        public float MinValue => _minValue;
        public float MaxValue => _maxValue;
        public string Unit => _unit;
        public string Title => _title;

        public void SetColorHex(string hex)
        {
            if (!SKColor.TryParse(hex, out SKColor parsed))
            {
                parsed = SKColors.Black;
                _colorHex = "#000000";
            }
            else
            {
                _colorHex = hex;
            }

            _activeColor = parsed;
            ComputeInactiveColor();
            StaticCacheValid = false;
        }

        public (SKColor Active, SKColor Inactive) Colors
        {
            get
            {
                return (_activeColor, _inactiveColor);
            }
        }

        /// <summary>
        /// Default background fill (black).
        /// </summary>
        protected void DrawBackground(SKCanvas canvas, int screenWidth, int screenHeight)
        {
            using SKPaint paint = new()
            {
                Style = SKPaintStyle.Fill,
                Color = SKColors.Black
            };
            canvas.DrawRect(new SKRect(0, 0, screenWidth, screenHeight), paint);
        }

        /// <summary>
        /// Compute a “dark” version (10% brightness) of the active color.
        /// </summary>
        private void ComputeInactiveColor()
        {
            // Reduce to 10%
            int r = (int)System.Math.Round(_activeColor.Red * 0.1f);
            int g = (int)System.Math.Round(_activeColor.Green * 0.1f);
            int b = (int)System.Math.Round(_activeColor.Blue * 0.1f);

            _inactiveColor = new SKColor((byte)r, (byte)g, (byte)b, _activeColor.Alpha);
        }
    }
}