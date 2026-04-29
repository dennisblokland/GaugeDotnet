using GaugeDotnet.Configuration;
using RG35XX.Core.GamePads;
using SkiaSharp;

namespace GaugeDotnet
{
    public class ConfigEditor
    {
        private enum EditorScreen
        {
            ScreenList,
            GaugeEdit,
            GridCellEdit
        }

        private readonly AppConfig _config;
        private readonly int _screenWidth;
        private readonly int _screenHeight;
        private readonly SKTypeface _font;

        private EditorScreen _currentScreen = EditorScreen.ScreenList;
        private int _cursorIndex;
        private int _selectedScreenIndex;
        private int _selectedCellIndex;
        private bool _saved;
        private double _savedMessageTimer;

        // Editable fields for a gauge (Type must be first, type-specific fields are filtered dynamically)
        private static readonly string[] AllGaugeFields =
        [
            "Type",
            "DataSource",
            "Title",
            "ColorHex",
            "MinValue",
            "MaxValue",
            "InitialValue",
            "Decimals",
            "SegmentCount",
            "Smoothing",
            "MaxDataPoints",
            "IntervalMs",
        ];

        private static readonly string[] ColorOptions =
        [
            "#00FFFF", "#FF0000", "#00FF00", "#FFFF00",
            "#FF00FF", "#FF8800", "#FFFFFF", "#0088FF",
        ];

        public bool IsActive { get; private set; } = true;
        public bool ConfigChanged { get; private set; }

        public ConfigEditor(AppConfig config, int screenWidth, int screenHeight)
        {
            _config = config;
            _screenWidth = screenWidth;
            _screenHeight = screenHeight;
            _font = FontHelper.GetFont("Race Sport");
        }

        private string[] GetFieldsForType(GaugeType type)
        {
            return type switch
            {
                GaugeType.Bar => ["Type", "DataSource", "Title", "ColorHex", "MinValue", "MaxValue", "InitialValue", "Decimals", "SegmentCount", "Smoothing"],
                GaugeType.Circular => ["Type", "DataSource", "Title", "ColorHex", "MinValue", "MaxValue", "InitialValue", "Decimals", "SegmentCount", "Smoothing"],
                GaugeType.Histogram => ["Type", "DataSource", "Title", "ColorHex", "MinValue", "MaxValue", "InitialValue", "Decimals", "MaxDataPoints", "IntervalMs"],
                GaugeType.Needle => ["Type", "DataSource", "Title", "ColorHex", "MinValue", "MaxValue", "InitialValue", "Decimals", "Smoothing"],
                GaugeType.Digital => ["Type", "DataSource", "Title", "ColorHex", "MinValue", "MaxValue", "InitialValue", "Decimals"],
                GaugeType.Sweep => ["Type", "DataSource", "Title", "ColorHex", "MinValue", "MaxValue", "InitialValue", "Decimals", "Smoothing"],
                GaugeType.MinMax => ["Type", "DataSource", "Title", "ColorHex", "MinValue", "MaxValue", "InitialValue", "Decimals", "SegmentCount", "Smoothing"],
                GaugeType.Grid => BuildGridFields(_config.Screens[_selectedScreenIndex].Gauge),
                _ => ["Type", "DataSource", "Title", "ColorHex", "MinValue", "MaxValue", "InitialValue", "Decimals"],
            };
        }

        private static string[] BuildGridFields(GaugeConfig gauge)
        {
            string[] fields = new string[1 + gauge.Cells.Count];
            fields[0] = "Type";
            for (int i = 0; i < gauge.Cells.Count; i++)
            {
                fields[i + 1] = $"Cell {i + 1}";
            }
            return fields;
        }

        public void HandleInput(GamepadKey key, SDL2.SDL.SDL_Keycode? sdlKey = null)
        {
            // Also handle keyboard arrow keys via SDL keycodes
            if (sdlKey.HasValue)
            {
                switch (sdlKey.Value)
                {
                    case SDL2.SDL.SDL_Keycode.SDLK_UP:
                        key = GamepadKey.UP;
                        break;
                    case SDL2.SDL.SDL_Keycode.SDLK_DOWN:
                        key = GamepadKey.DOWN;
                        break;
                    case SDL2.SDL.SDL_Keycode.SDLK_LEFT:
                        key = GamepadKey.LEFT;
                        break;
                    case SDL2.SDL.SDL_Keycode.SDLK_RIGHT:
                        key = GamepadKey.RIGHT;
                        break;
                    case SDL2.SDL.SDL_Keycode.SDLK_RETURN:
                    case SDL2.SDL.SDL_Keycode.SDLK_z:
                        key = GamepadKey.A_DOWN;
                        break;
                    case SDL2.SDL.SDL_Keycode.SDLK_ESCAPE:
                    case SDL2.SDL.SDL_Keycode.SDLK_x:
                        key = GamepadKey.B_DOWN;
                        break;
                    case SDL2.SDL.SDL_Keycode.SDLK_s:
                        key = GamepadKey.START_DOWN;
                        break;
                }
            }

            switch (_currentScreen)
            {
                case EditorScreen.ScreenList:
                    HandleScreenList(key);
                    break;
                case EditorScreen.GaugeEdit:
                    HandleGaugeEdit(key);
                    break;
                case EditorScreen.GridCellEdit:
                    HandleGridCellEdit(key);
                    break;
            }
        }

        public void Draw(SKCanvas canvas)
        {
            canvas.Clear(SKColors.Black);

            switch (_currentScreen)
            {
                case EditorScreen.ScreenList:
                    DrawScreenList(canvas);
                    break;
                case EditorScreen.GaugeEdit:
                    DrawGaugeEdit(canvas);
                    break;
                case EditorScreen.GridCellEdit:
                    DrawGridCellEdit(canvas);
                    break;
            }

            // Draw save confirmation
            if (_saved && _savedMessageTimer > 0)
            {
                using SKPaint savePaint = new() { Color = SKColors.LimeGreen, IsAntialias = true };
                using SKFont saveFont = new(_font, 20);
                canvas.DrawText("Config Saved!", _screenWidth / 2f - 60, _screenHeight - 20, saveFont, savePaint);
                _savedMessageTimer -= 0.016; // ~60fps
            }

            // Draw controls hint at bottom
            DrawControls(canvas);
        }

        private void HandleScreenList(GamepadKey key)
        {
            // Items: each screen + "Add Screen" option
            int itemCount = _config.Screens.Count + 1;

            switch (key)
            {
                case GamepadKey.UP:
                    _cursorIndex = Math.Max(0, _cursorIndex - 1);
                    break;
                case GamepadKey.DOWN:
                    _cursorIndex = Math.Min(itemCount - 1, _cursorIndex + 1);
                    break;
                case GamepadKey.A_DOWN:
                    if (_cursorIndex < _config.Screens.Count)
                    {
                        _selectedScreenIndex = _cursorIndex;
                        _currentScreen = EditorScreen.GaugeEdit;
                        _cursorIndex = 0;
                    }
                    else if (_config.Screens.Count < AppConfig.MaxScreens)
                    {
                        _config.Screens.Add(new ScreenConfig());
                        ConfigChanged = true;
                        _cursorIndex = _config.Screens.Count - 1;
                    }
                    break;
                case GamepadKey.B_DOWN:
                    IsActive = false;
                    break;
                case GamepadKey.START_DOWN:
                    SaveConfig();
                    break;
                case GamepadKey.X_DOWN:
                    // Delete screen
                    if (_cursorIndex < _config.Screens.Count && _config.Screens.Count > 1)
                    {
                        _config.Screens.RemoveAt(_cursorIndex);
                        ConfigChanged = true;
                        if (_cursorIndex >= _config.Screens.Count)
                        {
                            _cursorIndex = _config.Screens.Count;
                        }
                    }
                    break;
            }
        }

        private void HandleGaugeEdit(GamepadKey key)
        {
            GaugeConfig gauge = _config.Screens[_selectedScreenIndex].Gauge;
            string[] fields = GetFieldsForType(gauge.Type);

            switch (key)
            {
                case GamepadKey.UP:
                    _cursorIndex = Math.Max(0, _cursorIndex - 1);
                    break;
                case GamepadKey.DOWN:
                    _cursorIndex = Math.Min(fields.Length - 1, _cursorIndex + 1);
                    break;
                case GamepadKey.LEFT:
                case GamepadKey.RIGHT:
                    AdjustField(gauge, fields[_cursorIndex], key == GamepadKey.RIGHT ? 1 : -1);
                    ConfigChanged = true;
                    break;
                case GamepadKey.A_DOWN:
                    // Open cell edit if on a cell row
                    if (gauge.Type == GaugeType.Grid && fields[_cursorIndex].StartsWith("Cell "))
                    {
                        _selectedCellIndex = _cursorIndex - 1; // first field is Type
                        _currentScreen = EditorScreen.GridCellEdit;
                        _cursorIndex = 0;
                    }
                    break;
                case GamepadKey.B_DOWN:
                    _currentScreen = EditorScreen.ScreenList;
                    _cursorIndex = _selectedScreenIndex;
                    break;
                case GamepadKey.START_DOWN:
                    SaveConfig();
                    break;
            }
        }

        private static readonly string[] GridCellFields = ["DataSource", "Title", "Unit", "Decimals"];

        private static readonly string[] UnitOptions =
        [
            "", "rpm", "kPa", "\u00b0C", "\u00b0F", "V", "psi", "\u03bb", "AFR",
            "%", "\u00b0", "ms", "km/h", "mph", "mg/s", "g/s",
        ];

        private void HandleGridCellEdit(GamepadKey key)
        {
            GaugeConfig gauge = _config.Screens[_selectedScreenIndex].Gauge;
            GridCellConfig cell = gauge.Cells[_selectedCellIndex];

            switch (key)
            {
                case GamepadKey.UP:
                    _cursorIndex = Math.Max(0, _cursorIndex - 1);
                    break;
                case GamepadKey.DOWN:
                    _cursorIndex = Math.Min(GridCellFields.Length - 1, _cursorIndex + 1);
                    break;
                case GamepadKey.LEFT:
                case GamepadKey.RIGHT:
                    AdjustCellField(cell, GridCellFields[_cursorIndex], key == GamepadKey.RIGHT ? 1 : -1);
                    ConfigChanged = true;
                    break;
                case GamepadKey.B_DOWN:
                    _currentScreen = EditorScreen.GaugeEdit;
                    _cursorIndex = _selectedCellIndex + 1;
                    break;
                case GamepadKey.START_DOWN:
                    SaveConfig();
                    break;
            }
        }

        private static void AdjustCellField(GridCellConfig cell, string field, int direction)
        {
            List<string> dataSources = DataSourceMapper.DataSourceNames;

            switch (field)
            {
                case "DataSource":
                    int dsIdx = dataSources.IndexOf(cell.DataSource);
                    if (dsIdx < 0) dsIdx = 0;
                    dsIdx = (dsIdx + direction + dataSources.Count) % dataSources.Count;
                    cell.DataSource = dataSources[dsIdx];
                    break;
                case "Title":
                    int tIdx = dataSources.IndexOf(cell.Title);
                    if (tIdx < 0) tIdx = 0;
                    tIdx = (tIdx + direction + dataSources.Count) % dataSources.Count;
                    cell.Title = dataSources[tIdx];
                    break;
                case "Unit":
                    int uIdx = Array.IndexOf(UnitOptions, cell.Unit);
                    if (uIdx < 0) uIdx = 0;
                    uIdx = (uIdx + direction + UnitOptions.Length) % UnitOptions.Length;
                    cell.Unit = UnitOptions[uIdx];
                    break;
                case "Decimals":
                    cell.Decimals = Math.Clamp(cell.Decimals + direction, 0, 4);
                    break;
            }
        }

        private void AdjustField(GaugeConfig gauge, string field, int direction)
        {
            List<string> dataSources = DataSourceMapper.DataSourceNames;
            GaugeType[] gaugeTypes = (GaugeType[])Enum.GetValues(typeof(GaugeType));

            switch (field)
            {
                case "Type":
                    int typeIdx = Array.IndexOf(gaugeTypes, gauge.Type);
                    typeIdx = (typeIdx + direction + gaugeTypes.Length) % gaugeTypes.Length;
                    gauge.Type = gaugeTypes[typeIdx];
                    // Initialize 16 cells when switching to Grid
                    if (gauge.Type == GaugeType.Grid && gauge.Cells.Count < 4)
                    {
                        while (gauge.Cells.Count < 4)
                        {
                            gauge.Cells.Add(new GridCellConfig());
                        }
                    }
                    // Clamp cursor if new type has fewer fields
                    string[] newFields = GetFieldsForType(gauge.Type);
                    if (_cursorIndex >= newFields.Length)
                    {
                        _cursorIndex = newFields.Length - 1;
                    }
                    break;
                case "DataSource":
                    int dsIdx = dataSources.IndexOf(gauge.DataSource);
                    dsIdx = (dsIdx + direction + dataSources.Count) % dataSources.Count;
                    gauge.DataSource = dataSources[dsIdx];
                    break;
                case "Title":
                    int tIdx = dataSources.IndexOf(gauge.Title);
                    if (tIdx < 0) tIdx = 0;
                    tIdx = (tIdx + direction + dataSources.Count) % dataSources.Count;
                    gauge.Title = dataSources[tIdx];
                    break;
                case "ColorHex":
                    int cIdx = Array.IndexOf(ColorOptions, gauge.ColorHex);
                    if (cIdx < 0) cIdx = 0;
                    cIdx = (cIdx + direction + ColorOptions.Length) % ColorOptions.Length;
                    gauge.ColorHex = ColorOptions[cIdx];
                    break;
                case "MinValue":
                    gauge.MinValue += direction * 1;
                    break;
                case "MaxValue":
                    gauge.MaxValue += direction * 1;
                    break;
                case "InitialValue":
                    gauge.InitialValue += direction * 0.1f;
                    break;
                case "Decimals":
                    gauge.Decimals = Math.Clamp(gauge.Decimals + direction, 0, 4);
                    break;
                case "SegmentCount":
                    gauge.SegmentCount = Math.Clamp(gauge.SegmentCount + direction * 4, 4, 64);
                    break;
                case "Smoothing":
                    gauge.Smoothing = !gauge.Smoothing;
                    break;
                case "MaxDataPoints":
                    gauge.MaxDataPoints = Math.Clamp(gauge.MaxDataPoints + direction * 5, 10, 100);
                    break;
                case "IntervalMs":
                    gauge.IntervalMs = Math.Clamp(gauge.IntervalMs + direction * 100, 100, 5000);
                    break;
            }
        }

        private void SaveConfig()
        {
            ConfigService.Save(_config);
            _saved = true;
            _savedMessageTimer = 2.0;
            ConfigChanged = true;
        }

        private void DrawScreenList(SKCanvas canvas)
        {
            DrawTitle(canvas, "Screens");

            float y = 80;
            for (int i = 0; i < _config.Screens.Count; i++)
            {
                ScreenConfig screen = _config.Screens[i];
                string label = screen.Gauge.Type == GaugeType.Grid
                    ? $"Screen {i + 1}: Grid (2x2)"
                    : $"Screen {i + 1}: {screen.Gauge.Title} ({screen.Gauge.DataSource})";
                DrawMenuItem(canvas, label, y, i == _cursorIndex);
                y += 40;
            }

            if (_config.Screens.Count < AppConfig.MaxScreens)
            {
                DrawMenuItem(canvas, "+ Add Screen", y, _cursorIndex == _config.Screens.Count, SKColors.LimeGreen);
            }
        }

        private void DrawGaugeEdit(SKCanvas canvas)
        {
            GaugeConfig gauge = _config.Screens[_selectedScreenIndex].Gauge;
            DrawTitle(canvas, $"Screen {_selectedScreenIndex + 1}: {gauge.Title}");

            string[] fields = GetFieldsForType(gauge.Type);

            // For grid, scroll so cursor stays visible
            int maxVisible = (_screenHeight - 100) / 30;
            int scrollOffset = 0;
            if (fields.Length > maxVisible)
            {
                scrollOffset = Math.Max(0, _cursorIndex - maxVisible + 2);
                scrollOffset = Math.Min(scrollOffset, fields.Length - maxVisible);
            }

            float y = 70;
            for (int i = scrollOffset; i < fields.Length && y < _screenHeight - 40; i++)
            {
                string field = fields[i];
                string value = GetFieldValue(gauge, field);
                bool selected = i == _cursorIndex;

                DrawFieldRow(canvas, field, value, y, selected, field == "ColorHex" ? gauge.ColorHex : null);

                y += 30;
            }
        }

        private void DrawGridCellEdit(SKCanvas canvas)
        {
            GaugeConfig gauge = _config.Screens[_selectedScreenIndex].Gauge;
            GridCellConfig cell = gauge.Cells[_selectedCellIndex];
            DrawTitle(canvas, $"Cell {_selectedCellIndex + 1}");

            float y = 80;
            for (int i = 0; i < GridCellFields.Length; i++)
            {
                string field = GridCellFields[i];
                string value = field switch
                {
                    "DataSource" => cell.DataSource,
                    "Title" => cell.Title,
                    "Unit" => cell.Unit.Length == 0 ? "(none)" : cell.Unit,
                    "Decimals" => cell.Decimals.ToString(),
                    _ => "?"
                };
                bool selected = i == _cursorIndex;
                DrawFieldRow(canvas, field, value, y, selected, null);
                y += 38;
            }
        }

        private string GetFieldValue(GaugeConfig gauge, string field)
        {
            if (field.StartsWith("Cell ") && int.TryParse(field.AsSpan(5), out int cellNum))
            {
                int idx = cellNum - 1;
                if (idx >= 0 && idx < gauge.Cells.Count)
                {
                    GridCellConfig cell = gauge.Cells[idx];
                    return $"{cell.DataSource} ({cell.Unit})";
                }
                return "?";
            }

            return field switch
            {
                "Type" => gauge.Type.ToString(),
                "DataSource" => gauge.DataSource,
                "Title" => gauge.Title,
                "ColorHex" => gauge.ColorHex,
                "MinValue" => gauge.MinValue.ToString("F1"),
                "MaxValue" => gauge.MaxValue.ToString("F1"),
                "InitialValue" => gauge.InitialValue.ToString("F1"),
                "Decimals" => gauge.Decimals.ToString(),
                "SegmentCount" => gauge.SegmentCount.ToString(),
                "Smoothing" => gauge.Smoothing ? "ON" : "OFF",
                "MaxDataPoints" => gauge.MaxDataPoints.ToString(),
                "IntervalMs" => $"{gauge.IntervalMs}ms",
                _ => "?"
            };
        }

        private void DrawTitle(SKCanvas canvas, string title)
        {
            using SKPaint paint = new() { Color = SKColors.White, IsAntialias = true };
            using SKFont font = new(_font, 28);
            float textWidth = font.MeasureText(title);
            canvas.DrawText(title, (_screenWidth - textWidth) / 2, 40, font, paint);

            // Underline
            using SKPaint linePaint = new() { Color = new SKColor(60, 60, 60), StrokeWidth = 1 };
            canvas.DrawLine(40, 55, _screenWidth - 40, 55, linePaint);
        }

        private void DrawMenuItem(SKCanvas canvas, string label, float y, bool selected, SKColor? overrideColor = null)
        {
            SKColor color = overrideColor ?? SKColors.White;

            if (selected)
            {
                // Highlight background
                using SKPaint bgPaint = new() { Color = new SKColor(40, 40, 60), Style = SKPaintStyle.Fill };
                canvas.DrawRoundRect(30, y - 22, _screenWidth - 60, 34, 4, 4, bgPaint);

                // Cursor arrow
                using SKPaint arrowPaint = new() { Color = SKColors.Cyan, IsAntialias = true };
                using SKFont arrowFont = new(_font, 18);
                canvas.DrawText(">", 38, y, arrowFont, arrowPaint);
            }

            using SKPaint paint = new() { Color = selected ? SKColors.Cyan : color, IsAntialias = true };
            using SKFont font = new(_font, 18);
            canvas.DrawText(label, 60, y, font, paint);
        }

        private void DrawFieldRow(SKCanvas canvas, string label, string value, float y, bool selected, string? colorHex)
        {
            if (selected)
            {
                using SKPaint bgPaint = new() { Color = new SKColor(40, 40, 60), Style = SKPaintStyle.Fill };
                canvas.DrawRoundRect(30, y - 22, _screenWidth - 60, 34, 4, 4, bgPaint);

                using SKPaint arrowPaint = new() { Color = SKColors.Cyan, IsAntialias = true };
                using SKFont arrowFont = new(_font, 16);
                canvas.DrawText("<", 340, y, arrowFont, arrowPaint);
                canvas.DrawText(">", _screenWidth - 60, y, arrowFont, arrowPaint);
            }

            using SKPaint labelPaint = new() { Color = selected ? SKColors.Cyan : new SKColor(180, 180, 180), IsAntialias = true };
            using SKFont labelFont = new(_font, 16);
            canvas.DrawText(label, 60, y, labelFont, labelPaint);

            // Value on the right side
            SKColor valueColor = SKColors.White;
            if (colorHex != null && SKColor.TryParse(colorHex, out SKColor parsed))
            {
                valueColor = parsed;
            }

            using SKPaint valuePaint = new() { Color = selected ? valueColor : new SKColor(200, 200, 200), IsAntialias = true };
            using SKFont valueFont = new(_font, 16);
            float valueWidth = valueFont.MeasureText(value);
            canvas.DrawText(value, _screenWidth - 80 - valueWidth, y, valueFont, valuePaint);
        }

        private void DrawControls(SKCanvas canvas)
        {
            using SKPaint paint = new() { Color = new SKColor(100, 100, 100), IsAntialias = true };
            using SKFont font = new(_font, 12);

            string controls = _currentScreen switch
            {
                EditorScreen.ScreenList => "UP/DOWN:Nav  A:Edit  X:Delete  START:Save  B:Exit",
                EditorScreen.GaugeEdit => "UP/DOWN:Nav  LEFT/RIGHT:Change  A:Edit Cell  B:Back  START:Save",
                EditorScreen.GridCellEdit => "UP/DOWN:Nav  LEFT/RIGHT:Change  B:Back  START:Save",
                _ => ""
            };

            canvas.DrawText(controls, 30, _screenHeight - 10, font, paint);
        }
    }
}
