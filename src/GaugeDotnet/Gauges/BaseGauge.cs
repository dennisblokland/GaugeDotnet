using GaugeDotnet.Gauges.Models;
using SkiaSharp;
using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace GaugeDotnet.Gauges
{
    public abstract class BaseGauge
    {
        private decimal _value;
        private readonly decimal _minValue;
        private readonly decimal _maxValue;
        private string _colorHex = "#00FFFF";          // active color
        private string _inactiveColorHex = "#001919";  // computed dark variant
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

        public void SetValue(decimal v)
        {
            _value = v;
        }

        public decimal Value => _value;
        public decimal MinValue => _minValue;
        public decimal MaxValue => _maxValue;
        public string Unit => _unit;
        public string Title => _title;

        public void SetColorHex(string hex)
        {
            _colorHex = hex;
            ComputeInactiveColor();
            StaticCacheValid = false;
        }

        public (SKColor Active, SKColor Inactive) Colors
        {
            get
            {
                SKColor active = SKColor.Parse(_colorHex);
                SKColor inactive = SKColor.Parse(_inactiveColorHex);
                return (active, inactive);
            }
        }

        /// <summary>
        /// Default background fill (black).
        /// </summary>
        protected void DrawBackground(SKCanvas canvas, int screenWidth, int screenHeight)
        {
            SKPaint paint = new()
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
            // Strip “#” if present:
            string h = _colorHex.Replace("#", "");
            if (h.Length != 6)
            {
                _inactiveColorHex = "#000000";
                return;
            }

            if (!int.TryParse(h.Substring(0, 2), System.Globalization.NumberStyles.HexNumber, null, out int r)) r = 0;
            if (!int.TryParse(h.Substring(2, 2), System.Globalization.NumberStyles.HexNumber, null, out int g)) g = 0;
            if (!int.TryParse(h.Substring(4, 2), System.Globalization.NumberStyles.HexNumber, null, out int b)) b = 0;

            // Reduce to 10%
            r = (int)System.Math.Round(r * 0.1f);
            g = (int)System.Math.Round(g * 0.1f);
            b = (int)System.Math.Round(b * 0.1f);

            _inactiveColorHex = $"#{r:X2}{g:X2}{b:X2}";
        }
    }
}