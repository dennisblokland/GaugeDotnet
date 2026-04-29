using GaugeDotnet.Configuration;
using GaugeDotnet.Gauges;
using SkiaSharp;
using GaugeDotnet;
using static SDL2.SDL;
using RG35XX.Libraries;
using RG35XX.Core.Interfaces;
using RG35XX.Core.GamePads;

internal class Program
{
    private static void ShowErrorScreen(string message)
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

    private static async Task<IMeDevice?> ConnectDeviceAsync(AppConfig appConfig, CancellationTokenSource exit)
    {
        if (appConfig.DemoMode)
        {
            Console.WriteLine("[DemoMode] Using SimulatedMeDevice - no BLE required");
            SimulatedMeDevice simulatedDevice = new();
            await simulatedDevice.ConnectAsync();
            return simulatedDevice;
        }

        exit.CancelAfter(TimeSpan.FromSeconds(60));
        Console.WriteLine("Searching for Bluetooth adapter...");

        try
        {
            BleManager bleManager = await BleManager.CreateAsync();
            Console.WriteLine("Scanning for ME device...");

            IMeDevice? device = await bleManager.ScanAsync(findAll: true, cancellationToken: exit.Token);
            if (device == null)
            {
                Console.WriteLine("No ME device found.");
                ShowErrorScreen("No ME device found.");
                return null;
            }

            await device.ConnectAsync();
            Console.WriteLine("ME device connected.");
            return device;
        }
        catch (DllNotFoundException ex)
        {
            Console.WriteLine($"BLE native library missing: {ex.Message}");
            ShowErrorScreen("BLE library not available\nfor this platform.");
            return null;
        }
        catch (BadImageFormatException ex)
        {
            Console.WriteLine($"BLE library architecture mismatch: {ex.Message}");
            ShowErrorScreen("BLE library architecture\nmismatch.");
            return null;
        }
        catch (Exception ex) when (ex.GetType().Name.Contains("Btle"))
        {
            Console.WriteLine($"Bluetooth error: {ex.Message}");
            ShowErrorScreen($"Bluetooth error:\n{ex.Message}");
            return null;
        }
    }

    private static async Task Main(string[] args)
    {
        using CancellationTokenSource exit = new();

        AppConfig appConfig = ConfigService.Load();
        Console.WriteLine($"Loaded {appConfig.Screens.Count} screen(s) from config");

        IMeDevice? meDevice = await ConnectDeviceAsync(appConfig, exit);
        if (meDevice == null && !appConfig.DemoMode)
        {
            return;
        }

        GameLoop gameLoop = new(appConfig, meDevice, screenWidth: 640, screenHeight: 480);
        gameLoop.Run();

        exit.Cancel();
        SDL_Quit();
        if (meDevice is IDisposable disposable)
        {
            disposable.Dispose();
        }
        Console.WriteLine("Exiting program...");
        Environment.Exit(0);
    }
}