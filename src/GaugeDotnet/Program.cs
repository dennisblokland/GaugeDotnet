using GaugeDotnet.Gauges.Models;
using GaugeDotnet.Gauges;
using SkiaSharp;
using GaugeDotnet;
using static SDL2.SDL;
using RG35XX.Libraries;
using RG35XX.Core.Interfaces;
using RG35XX.Core.GamePads;
using ME1_4NET;

internal class Program
{
    private static MeDevice? meDevice; // Make it static or keep it referenced globally

    private static async Task Main(string[] args)
    {
        // Define a CancellationTokenSource for cancellation support
        using var exit = new CancellationTokenSource();
        exit.CancelAfter(TimeSpan.FromSeconds(60)); // Auto-cancel after 30 seconds if no device is found

        BleManager bleManager = await BleManager.CreateAsync();
        meDevice = await bleManager.ScanAsync(findAll: true, cancellationToken: exit.Token);
        if (meDevice == null)
        {
            Console.WriteLine("No ME device found. Exiting...");
            return;
        }
       
        await meDevice.ConnectAsync();



        int screenWidth = 640;
        int screenHeight = 480;
        IGamePadReader gamePadReader = new GamePadReader();
        gamePadReader.Initialize();
        GaugeSDL gaugeSDL = new(
            screenWidth: screenWidth,
            screenHeight: screenHeight
        );

        GRGlFramebufferInfo glFramebufferInfo = new(
            /*fboId=*/   0u,
            /*format=*/  SKColorType.Rgba8888.ToGlSizedFormat()
        );

        GRBackendRenderTarget backendRenderTarget = new(
            screenWidth,
            screenHeight,
            /*sampleCount=*/    0,
            /*stencilBits=*/    8,
            glFramebufferInfo
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
        double GetFps()
        {
            frameCount++;
            double elapsed = stopwatch.Elapsed.TotalSeconds;
            if (elapsed - lastReport >= 1.0)
            {
                currentFps = frameCount / (elapsed - lastReport);
                lastReport = elapsed;
                frameCount = 0;
            }
            return currentFps;
        }

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
                gauge.SetValue(meDevice.afr);
                lastUpdate = now;
            }

            gauge.Draw(canvas);

            //  (Optional) Draw an FPS counter in the top‐left corner
            using SKPaint textPaint = new();
            using SKFont font = new();

            textPaint.Color = SKColors.White;
            textPaint.IsAntialias = true;
            font.Size = 20;

            // Measure “FPS: XXX” for width (if needed)
            string fpsText = $"FPS: {GetFps():F2}";
            canvas.DrawText(fpsText, 10, 25, font, textPaint);

            gaugeSDL.FlushAndSwap();
        }

        exit.Cancel();
        SDL_Quit();
        Console.WriteLine("Exiting program...");
        Environment.Exit(0);
    }
}