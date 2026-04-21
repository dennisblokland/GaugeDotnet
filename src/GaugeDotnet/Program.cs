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

        BleManager bleManager = await BleManager.CreateAsync();
        meDevice = await bleManager.ScanAsync(findAll: true, cancellationToken: exit.Token);
        if (meDevice == null)
        {
            Console.WriteLine("No ME device found. Exiting...");
            return;
        }

        await meDevice.ConnectAsync();
#endif



        int screenWidth = 640;
        int screenHeight = 480;
        IGamePadReader gamePadReader = new GamePadReader();
        gamePadReader.Initialize();
        GaugeSDL gaugeSDL = new(
            screenWidth: screenWidth,
            screenHeight: screenHeight
        );

        // Prepare settings:
        BarGaugeSettings settings = new()
        {
            InitialValue = 14.7M,
            MinValue = 8,
            MaxValue = 18,
            Unit = "",
            Title = "AFR",
            Decimals = 2,

        };

        // Instantiate the BarGauge:
        BarGauge gauge = new(
            settings: settings,
            screenWidth: screenWidth,
            screenHeight: screenHeight
        );

        bool running = true;
        SDL_Event e;

        gauge.SetColorHex("#00FFFF"); // active segments in bright green

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
        while (running)
        {

            // ‣ Poll SDL events
            while (SDL_PollEvent(out e) == 1)
            {
                switch (e.type)
                {
                    case SDL_EventType.SDL_QUIT:
                        running = false;
                        break;
                    case SDL_EventType.SDL_KEYDOWN:
                        {
                            // Key was pressed
                            SDL_KeyboardEvent keyEvent = e.key;
                            SDL_Keycode keycode = keyEvent.keysym.sym;
                            byte repeat = keyEvent.repeat; // 0 if not a repeat
                            KeyBus.OnKeyDown(keycode);
                        }
                        break;

                    case SDL_EventType.SDL_KEYUP:
                        {
                            // Key was released
                            SDL_KeyboardEvent keyEvent = e.key;
                            SDL_Keycode keycode = keyEvent.keysym.sym;
                            KeyBus.OnKeyUp(keycode);
                        }
                        break;
                }
            }
            GamepadKey key = gamePadReader.ReadInput();
            if (key == GamepadKey.MENU_DOWN)
            {
                running = false;
            }

            SKCanvas canvas = gaugeSDL.GetCanvas();

            // Clear to dark gray
            canvas.Clear(new SKColor(0, 0, 0));

            // // Update the gauge value every second

            double now = stopwatch.Elapsed.TotalSeconds;
            if (now - lastUpdate >= 0.05)
            {
              if(meDevice != null && meDevice.IsConnected){
                gauge.SetValue(meDevice.afr);
              }

                lastUpdate = now;
            }

            gauge.Draw(canvas);

            // Draw an FPS counter in the top-left corner.
            GetFps();
            canvas.DrawText(fpsText, 10, 25, fpsFont, fpsPaint);

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