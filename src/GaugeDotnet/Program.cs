using GaugeDotnet.Configuration;
using GaugeDotnet.Devices;
using GaugeDotnet.Rendering;
using GaugeDotnet;
using static SDL2.SDL;

internal class Program
{
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
            BleManager bleManager = BleManager.Create();
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
                ErrorScreen.Show("No ME device found.");
                return null;
            }

            await device.ConnectAsync();
            Console.WriteLine("ME device connected.");
            return device;
        }
        catch (DllNotFoundException ex)
        {
            Console.WriteLine($"BLE native library missing: {ex.Message}");
            ErrorScreen.Show("BLE library not available\nfor this platform.");
            return null;
        }
        catch (BadImageFormatException ex)
        {
            Console.WriteLine($"BLE library architecture mismatch: {ex.Message}");
            ErrorScreen.Show("BLE library architecture\nmismatch.");
            return null;
        }
        catch (FormatException ex)
        {
            Console.WriteLine($"Invalid configured Bluetooth MAC address: {ex.Message}");
            ErrorScreen.Show("Invalid Bluetooth MAC\naddress in config.");
            return null;
        }
        catch (Exception ex) when (ex.GetType().Name.Contains("Btle"))
        {
            Console.WriteLine($"Bluetooth error: {ex.Message}");
            ErrorScreen.Show($"Bluetooth error:\n{ex.Message}");
            return null;
        }
        catch (OperationCanceledException)
        {
            Console.WriteLine("Timed out while scanning for ME device.");
            ErrorScreen.Show("Timed out while scanning\nfor ME device.");
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

        using GameLoop gameLoop = new(appConfig, meDevice, screenWidth: 640, screenHeight: 480);
        gameLoop.Run();

        exit.Cancel();
        if (meDevice is IDisposable disposable)
        {
            disposable.Dispose();
        }
        SDL_Quit();
        Console.WriteLine("Exiting program...");
        Environment.Exit(0);
    }
}
