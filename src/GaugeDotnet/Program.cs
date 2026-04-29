using GaugeDotnet.Configuration;
using GaugeDotnet.Gauges.Models;
using GaugeDotnet.Gauges;
using SkiaSharp;
using GaugeDotnet;
using static SDL2.SDL;
using RG35XX.Libraries;
using RG35XX.Core.Interfaces;
using RG35XX.Core.GamePads;

internal class Program
{
    private static IMeDevice? meDevice;

    private static List<(BaseGauge Gauge, string DataSource)> BuildScreens(AppConfig config, int screenWidth, int screenHeight)
    {
        List<(BaseGauge Gauge, string DataSource)> screens = new();
        foreach (ScreenConfig screenConfig in config.Screens)
        {
            GaugeConfig gaugeConfig = screenConfig.Gauge;
            BaseGauge gauge;
            switch (gaugeConfig.Type)
            {
                case GaugeType.Circular:
                    CircularGaugeSettings circularSettings = new()
                    {
                        InitialValue = gaugeConfig.InitialValue,
                        MinValue = gaugeConfig.MinValue,
                        MaxValue = gaugeConfig.MaxValue,
                        Unit = gaugeConfig.Unit,
                        Title = gaugeConfig.Title,
                        Decimals = gaugeConfig.Decimals,
                        SegmentCount = gaugeConfig.SegmentCount,
                        Smoothing = gaugeConfig.Smoothing,
                    };
                    gauge = new CircularGauge(
                        settings: circularSettings,
                        screenWidth: screenWidth,
                        screenHeight: screenHeight
                    );
                    break;

                case GaugeType.Histogram:
                    HistogramGaugeSettings histogramSettings = new()
                    {
                        InitialValue = gaugeConfig.InitialValue,
                        MinValue = gaugeConfig.MinValue,
                        MaxValue = gaugeConfig.MaxValue,
                        Unit = gaugeConfig.Unit,
                        Title = gaugeConfig.Title,
                        Decimals = gaugeConfig.Decimals,
                        MaxDataPoints = gaugeConfig.MaxDataPoints,
                        IntervalMs = gaugeConfig.IntervalMs,
                    };
                    gauge = new HistogramGauge(
                        settings: histogramSettings,
                        screenWidth: screenWidth,
                        screenHeight: screenHeight
                    );
                    break;

                case GaugeType.Needle:
                    NeedleGaugeSettings needleSettings = new()
                    {
                        InitialValue = gaugeConfig.InitialValue,
                        MinValue = gaugeConfig.MinValue,
                        MaxValue = gaugeConfig.MaxValue,
                        Unit = gaugeConfig.Unit,
                        Title = gaugeConfig.Title,
                        Decimals = gaugeConfig.Decimals,
                        Smoothing = gaugeConfig.Smoothing,
                    };
                    gauge = new NeedleGauge(
                        settings: needleSettings,
                        screenWidth: screenWidth,
                        screenHeight: screenHeight
                    );
                    break;

                case GaugeType.Digital:
                    DigitalGaugeSettings digitalSettings = new()
                    {
                        InitialValue = gaugeConfig.InitialValue,
                        MinValue = gaugeConfig.MinValue,
                        MaxValue = gaugeConfig.MaxValue,
                        Unit = gaugeConfig.Unit,
                        Title = gaugeConfig.Title,
                        Decimals = gaugeConfig.Decimals,
                    };
                    gauge = new DigitalGauge(
                        settings: digitalSettings,
                        screenWidth: screenWidth,
                        screenHeight: screenHeight
                    );
                    break;

                case GaugeType.Sweep:
                    SweepGaugeSettings sweepSettings = new()
                    {
                        InitialValue = gaugeConfig.InitialValue,
                        MinValue = gaugeConfig.MinValue,
                        MaxValue = gaugeConfig.MaxValue,
                        Unit = gaugeConfig.Unit,
                        Title = gaugeConfig.Title,
                        Decimals = gaugeConfig.Decimals,
                        Smoothing = gaugeConfig.Smoothing,
                    };
                    gauge = new SweepGauge(
                        settings: sweepSettings,
                        screenWidth: screenWidth,
                        screenHeight: screenHeight
                    );
                    break;

                case GaugeType.MinMax:
                    MinMaxGaugeSettings minMaxSettings = new()
                    {
                        InitialValue = gaugeConfig.InitialValue,
                        MinValue = gaugeConfig.MinValue,
                        MaxValue = gaugeConfig.MaxValue,
                        Unit = gaugeConfig.Unit,
                        Title = gaugeConfig.Title,
                        Decimals = gaugeConfig.Decimals,
                        SegmentCount = gaugeConfig.SegmentCount,
                        Smoothing = gaugeConfig.Smoothing,
                    };
                    gauge = new MinMaxGauge(
                        settings: minMaxSettings,
                        screenWidth: screenWidth,
                        screenHeight: screenHeight
                    );
                    break;

                case GaugeType.Bar:
                default:
                    BarGaugeSettings barSettings = new()
                    {
                        InitialValue = gaugeConfig.InitialValue,
                        MinValue = gaugeConfig.MinValue,
                        MaxValue = gaugeConfig.MaxValue,
                        Unit = gaugeConfig.Unit,
                        Title = gaugeConfig.Title,
                        Decimals = gaugeConfig.Decimals,
                        SegmentCount = gaugeConfig.SegmentCount,
                        Smoothing = gaugeConfig.Smoothing,
                    };
                    gauge = new BarGauge(
                        settings: barSettings,
                        screenWidth: screenWidth,
                        screenHeight: screenHeight
                    );
                    break;
            }

            gauge.SetColorHex(gaugeConfig.ColorHex);
            screens.Add((gauge, gaugeConfig.DataSource));
        }
        return screens;
    }

    private static void ShowErrorScreen(string message)
    {
        int w = 640, h = 480;
        GaugeSDL sdl = new(screenWidth: w, screenHeight: h);
        IGamePadReader gp = new GamePadReader();
        gp.Initialize();

        // Render text to a CPU bitmap, then blit to GPU canvas
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

        // "Press any button to exit" in white
        paint.Color = SKColors.White;
        const string exitMsg = "Press any button to exit";
        float ew = font.MeasureText(exitMsg);
        bmpCanvas.DrawText(exitMsg, (w - ew) / 2, y + 20, font, paint);

        SKCanvas canvas = sdl.GetCanvas();
        canvas.Clear(SKColors.Black);
        using var image = SKImage.FromBitmap(bitmap);
        canvas.DrawImage(image, 0, 0);
        sdl.FlushAndSwap();

        // Wait for button/key press
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

    private static async Task Main(string[] args)
    {
        // Define a CancellationTokenSource for cancellation support
        using var exit = new CancellationTokenSource();

#if DEBUG
        // In debug mode bypass BLE entirely and use a simulated device
        Console.WriteLine("[DEBUG] Using SimulatedMeDevice - no BLE required");
        var simulatedDevice = new SimulatedMeDevice();
        meDevice = simulatedDevice;
        await meDevice.ConnectAsync();
#else
        exit.CancelAfter(TimeSpan.FromSeconds(60)); // Auto-cancel after 60 seconds if no device is found

        Console.WriteLine("Searching for Bluetooth adapter...");

        try
        {
            BleManager bleManager = await BleManager.CreateAsync();

            Console.WriteLine("Scanning for ME device...");

            meDevice = await bleManager.ScanAsync(findAll: true, cancellationToken: exit.Token);
            if (meDevice == null)
            {
                Console.WriteLine("No ME device found.");
                ShowErrorScreen("No ME device found.");
                return;
            }

            await meDevice.ConnectAsync();
            Console.WriteLine("ME device connected.");
        }
        catch (DllNotFoundException ex)
        {
            Console.WriteLine($"BLE native library missing: {ex.Message}");
            ShowErrorScreen("BLE library not available\nfor this platform.");
            return;
        }
        catch (BadImageFormatException ex)
        {
            Console.WriteLine($"BLE library architecture mismatch: {ex.Message}");
            ShowErrorScreen("BLE library architecture\nmismatch.");
            return;
        }
        catch (Exception ex) when (ex.GetType().Name.Contains("Btle"))
        {
            Console.WriteLine($"Bluetooth error: {ex.Message}");
            ShowErrorScreen($"Bluetooth error:\n{ex.Message}");
            return;
        }
#endif

        int screenWidth = 640;
        int screenHeight = 480;
        IGamePadReader gamePadReader = new GamePadReader();
        gamePadReader.Initialize();
        GaugeSDL gaugeSDL = new(
            screenWidth: screenWidth,
            screenHeight: screenHeight
        );

        // Load configuration (creates default gauges.json if missing)
        AppConfig appConfig = ConfigService.Load();
        Console.WriteLine($"Loaded {appConfig.Screens.Count} screen(s) from config");

        // Build gauge instances per screen
        List<(BaseGauge Gauge, string DataSource)> screens = BuildScreens(appConfig, screenWidth, screenHeight);

        int currentScreen = 0;
        ConfigEditor? configEditor = null;

        bool running = true;
        SDL_Event e;

        System.Diagnostics.Stopwatch stopwatch = System.Diagnostics.Stopwatch.StartNew();
        int frameCount = 0;
        double lastReport = 0.0;
        double currentFps = 0.0;
        string fpsText = "FPS: 0.00";
        double GetFps()
        {
            frameCount++;
            double elapsed = stopwatch.Elapsed.TotalSeconds;
            if (elapsed - lastReport >= 1.0)
            {
                currentFps = frameCount / (elapsed - lastReport);
                lastReport = elapsed;
                frameCount = 0;
                fpsText = $"FPS: {currentFps:F2}";
            }
            return currentFps;
        }

        using SKPaint fpsPaint = new()
        {
            Color = SKColors.White,
            IsAntialias = true
        };
        using SKFont fpsFont = new() { Size = 20 };

        double lastUpdate = 0;
        SDL2.SDL.SDL_Keycode? lastSdlKey = null;
        while (running)
        {
            lastSdlKey = null;

            // Poll SDL events
            while (SDL_PollEvent(out e) == 1)
            {
                switch (e.type)
                {
                    case SDL_EventType.SDL_QUIT:
                        running = false;
                        break;
                    case SDL_EventType.SDL_KEYDOWN:
                        {
                            SDL_KeyboardEvent keyEvent = e.key;
                            SDL_Keycode keycode = keyEvent.keysym.sym;
                            byte repeat = keyEvent.repeat;
                            if (repeat == 0)
                            {
                                lastSdlKey = keycode;
                            }
                            KeyBus.OnKeyDown(keycode);
                        }
                        break;

                    case SDL_EventType.SDL_KEYUP:
                        {
                            SDL_KeyboardEvent keyEvent = e.key;
                            SDL_Keycode keycode = keyEvent.keysym.sym;
                            KeyBus.OnKeyUp(keycode);
                        }
                        break;
                }
            }
            GamepadKey key = gamePadReader.ReadInput();

            // Config editor mode
            if (configEditor != null)
            {
                configEditor.HandleInput(key, lastSdlKey);

                if (!configEditor.IsActive)
                {
                    // Exiting editor - rebuild gauges if config changed
                    if (configEditor.ConfigChanged)
                    {
                        screens = BuildScreens(appConfig, screenWidth, screenHeight);
                        if (currentScreen >= screens.Count)
                        {
                            currentScreen = Math.Max(0, screens.Count - 1);
                        }
                    }
                    configEditor = null;
                }
                else
                {
                    SKCanvas canvas = gaugeSDL.GetCanvas();
                    configEditor.Draw(canvas);
                    gaugeSDL.FlushAndSwap();
                    continue;
                }
            }

            if (key == GamepadKey.MENU_DOWN)
            {
                running = false;
            }
            else if (key == GamepadKey.SELECT_DOWN || lastSdlKey == SDL_Keycode.SDLK_TAB)
            {
                configEditor = new ConfigEditor(appConfig, screenWidth, screenHeight);
                continue;
            }
            else if (key == GamepadKey.RIGHT && screens.Count > 1)
            {
                currentScreen = (currentScreen + 1) % screens.Count;
            }
            else if (key == GamepadKey.LEFT && screens.Count > 1)
            {
                currentScreen = (currentScreen - 1 + screens.Count) % screens.Count;
            }

            SKCanvas canvas2 = gaugeSDL.GetCanvas();

            // Clear to black
            canvas2.Clear(new SKColor(0, 0, 0));

            double now = stopwatch.Elapsed.TotalSeconds;
            if (now - lastUpdate >= 0.05)
            {
                if (meDevice != null && meDevice.IsConnected && screens.Count > 0)
                {
                    (BaseGauge g, string dataSource) = screens[currentScreen];
                    decimal value = DataSourceMapper.ReadValue(meDevice.Data, dataSource);
                    g.SetValue(value);
                }

                lastUpdate = now;
            }

            if (screens.Count > 0 && currentScreen < screens.Count)
            {
                screens[currentScreen].Gauge.Draw(canvas2);
            }

            // Draw an FPS counter in the top-left corner.
            GetFps();
            canvas2.DrawText(fpsText, 10, 25, fpsFont, fpsPaint);

            gaugeSDL.FlushAndSwap();
        }

        exit.Cancel();
        SDL_Quit();
#if DEBUG
        (meDevice as IDisposable)?.Dispose();
#endif
        Console.WriteLine("Exiting program...");
        Environment.Exit(0);
    }
}