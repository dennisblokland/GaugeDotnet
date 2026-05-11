using GaugeDotnet.Configuration;
using GaugeDotnet.Devices;
using GaugeDotnet.Rendering;
using GaugeDotnet;
using RG35XX.Libraries;
using static SDL2.SDL;

internal class Program
{
    private record StartupResult(IMeDevice? Device, string? ErrorMessage);

    private static async Task<StartupResult> StartupAsync(AppConfig appConfig, BleManager? bleManager, CancellationTokenSource exit)
    {
        if (appConfig.DemoMode)
        {
            Console.WriteLine("[DemoMode] Using SimulatedMeDevice - no BLE required");
            SimulatedMeDevice simulatedDevice = new();
            await simulatedDevice.ConnectAsync();
            return new StartupResult(simulatedDevice, null);
        }

        if (bleManager == null)
        {
            return new StartupResult(null, "BLE library not available\nfor this platform.");
        }

        exit.CancelAfter(TimeSpan.FromSeconds(60));
        Console.WriteLine("Searching for Bluetooth adapter...");

        try
        {
            IMeDevice? device;

            if (!string.IsNullOrWhiteSpace(appConfig.DeviceMacAddress))
            {
                Console.WriteLine($"Connecting to configured ME device {appConfig.DeviceMacAddress}...");
                device = await bleManager.ConnectByAddressAsync(appConfig.DeviceMacAddress, exit.Token);
            }
            else
            {
                Console.WriteLine("Scanning for ME device...");
                device = await bleManager.ScanAsync(findAll: true, cancellationToken: exit.Token);
            }

            if (device == null)
            {
                Console.WriteLine("No ME device found.");
                return new StartupResult(null, "No ME device found.");
            }

            await device.ConnectAsync();
            Console.WriteLine("ME device connected.");
            return new StartupResult(device, null);
        }
        catch (FormatException ex)
        {
            Console.WriteLine($"Invalid configured Bluetooth MAC address: {ex.Message}");
            return new StartupResult(null, "Invalid Bluetooth MAC\naddress in config.");
        }
        catch (Exception ex) when (ex.GetType().Name.Contains("Btle"))
        {
            Console.WriteLine($"Bluetooth error: {ex.Message}");
            return new StartupResult(null, $"Bluetooth error:\n{ex.Message}");
        }
        catch (OperationCanceledException)
        {
            Console.WriteLine("Timed out while scanning for ME device.");
            return new StartupResult(null, "Timed out while scanning\nfor ME device.");
        }
    }

    private static async Task Main(string[] args)
    {
        using CancellationTokenSource exit = new();

        AppConfig appConfig = ConfigService.Load();
        Console.WriteLine($"Loaded {appConfig.Screens.Count} screen(s) from config");

        if (!appConfig.DemoMode)
            await BluetoothHardwareInit.InitializeAsync(exit.Token);

        // BleManager owns native callback delegates; keep it rooted for the entire program lifetime
        // so the native side never invokes a GC'd PeripheralFoundCallback after scan cancellation.
        BleManager? bleManager = null;
        string? bleInitError = null;
        if (!appConfig.DemoMode)
        {
            try
            {
                bleManager = BleManager.Create();
            }
            catch (DllNotFoundException ex)
            {
                Console.WriteLine($"BLE native library missing: {ex.Message}");
                bleInitError = "BLE library not available\nfor this platform.";
            }
            catch (BadImageFormatException ex)
            {
                Console.WriteLine($"BLE library architecture mismatch: {ex.Message}");
                bleInitError = "BLE library architecture\nmismatch.";
            }
        }

        GaugeSDL sdl = new(screenWidth: 640, screenHeight: 480);

        try
        {
            if (bleInitError != null)
            {
                ErrorScreen.ShowOn(sdl, bleInitError);
                return;
            }

            Task<StartupResult> startupTask = StartupAsync(appConfig, bleManager, exit);
            SplashScreen.ShowUntil(sdl, startupTask);
            StartupResult result = await startupTask;

            if (result.ErrorMessage != null)
            {
                ErrorScreen.ShowOn(sdl, result.ErrorMessage);
                return;
            }

            if (result.Device == null && !appConfig.DemoMode)
            {
                return;
            }

            using (GameLoop gameLoop = new(appConfig, result.Device, sdl, screenWidth: 640, screenHeight: 480))
            {
                gameLoop.Run();
            }

            exit.Cancel();
            if (result.Device is IDisposable disposable)
            {
                disposable.Dispose();
            }
        }
        finally
        {
            sdl.Dispose();
            bleManager?.Dispose();
            GC.KeepAlive(bleManager);
            SDL_Quit();
        }

        Console.WriteLine("Exiting program...");
        Environment.Exit(0);
    }
}
