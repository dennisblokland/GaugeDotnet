using System.Diagnostics;

namespace RG35XX.Libraries
{
    public static class BluetoothHardwareInit
    {
        public static async Task InitializeAsync(CancellationToken cancellationToken = default)
        {
            await EnsureDbusAsync(cancellationToken);
            EnsureRtlBtlpm();
            EnsureRtkHciattach();
            await WaitForHci0Async(cancellationToken);
            await ForceHci0UpAsync(cancellationToken);
            await EnsureBluetoothdAsync(cancellationToken);
            await PowerOnAdapterAsync(cancellationToken);

            string hciStatus = Run("hciconfig", "hci0").Output.Split('\n').FirstOrDefault() ?? "";
            Console.WriteLine($"[BLE] hci0: {hciStatus}");
            Console.WriteLine($"[BLE] bluetoothd pid: {(IsRunning("bluetoothd") ? "running" : "NOT RUNNING")}");
        }

        private static async Task EnsureDbusAsync(CancellationToken ct)
        {
            if (IsRunning("dbus-daemon"))
                return;

            Directory.CreateDirectory("/run/dbus");
            Run("dbus-daemon", "--system --fork");

            for (int i = 1; i <= 5; i++)
            {
                if (File.Exists("/run/dbus/system_bus_socket")) return;
                Console.WriteLine($"[BLE] waiting for D-Bus system socket ({i}/5)");
                await Task.Delay(1000, ct);
            }
        }

        private static void EnsureRtlBtlpm()
        {
            if (IsModuleLoaded("rtl_btlpm")) return;

            string kernelRelease = Run("uname", "-r").Output.Trim();
            string koPath = $"/lib/modules/{kernelRelease}/kernel/drivers/bluetooth/rtl_btlpm.ko";

            if (File.Exists(koPath))
                Run("modprobe", koPath);
            else
                Run("modprobe", "rtl_btlpm");
        }

        private static void EnsureRtkHciattach()
        {
            if (Run("pgrep", "-f rtk_hciattach").ExitCode == 0)
            {
                Console.WriteLine("[BLE] rtk_hciattach already running");
                return;
            }

            Console.WriteLine("[BLE] starting rtk_hciattach");
            StartBackground("rtk_hciattach", "-n -s 115200 /dev/ttyS1 rtk_h5");
        }

        private static async Task WaitForHci0Async(CancellationToken ct)
        {
            for (int i = 1; i <= 15; i++)
            {
                if (Run("hciconfig", "hci0").ExitCode == 0) return;
                Console.WriteLine($"[BLE] waiting for hci0 ({i}/15)");
                await Task.Delay(1000, ct);
            }
        }

        private static async Task ForceHci0UpAsync(CancellationToken ct)
        {
            for (int i = 1; i <= 5; i++)
            {
                Run("hciconfig", "hci0 up");
                if (Run("hciconfig", "hci0").Output.Contains("UP")) return;
                Console.WriteLine($"[BLE] waiting for hci0 UP ({i}/5)");
                await Task.Delay(1000, ct);
            }
        }

        private static async Task EnsureBluetoothdAsync(CancellationToken ct)
        {
            if (!IsRunning("bluetoothd"))
            {
                Console.WriteLine("[BLE] starting bluetoothd");
                StartBackground("/usr/libexec/bluetooth/bluetoothd", "-n -d");
            }
            else
            {
                Console.WriteLine("[BLE] bluetoothd already running");
            }

            for (int i = 1; i <= 5; i++)
            {
                if (IsRunning("bluetoothd")) return;
                Console.WriteLine($"[BLE] waiting for bluetoothd ({i}/5)");
                await Task.Delay(1000, ct);
            }
        }

        private static async Task PowerOnAdapterAsync(CancellationToken ct)
        {
            Console.WriteLine("[BLE] powering adapter on through bluetoothctl");
            Run("bluetoothctl", "power on");
            await Task.Delay(1000, ct);
        }

        private static bool IsRunning(string name) =>
            Process.GetProcessesByName(name).Length > 0;

        private static bool IsModuleLoaded(string name)
        {
            try
            {
                return File.ReadLines("/proc/modules")
                    .Any(l => l.StartsWith(name + " ", StringComparison.Ordinal));
            }
            catch { return false; }
        }

        private static (int ExitCode, string Output) Run(string cmd, string args = "")
        {
            try
            {
                using Process p = new()
                {
                    StartInfo = new ProcessStartInfo(cmd, args)
                    {
                        UseShellExecute = false,
                        RedirectStandardOutput = true,
                        RedirectStandardError = true,
                    }
                };
                p.Start();
                string output = p.StandardOutput.ReadToEnd();
                p.WaitForExit(5000);
                return (p.ExitCode, output);
            }
            catch (Exception ex)
            {
                Console.WriteLine($"[BLE] '{cmd} {args}' failed: {ex.Message}");
                return (-1, string.Empty);
            }
        }

        private static void StartBackground(string cmd, string args)
        {
            try
            {
                Process.Start(new ProcessStartInfo(cmd, args)
                {
                    UseShellExecute = false,
                });
            }
            catch (Exception ex)
            {
                Console.WriteLine($"[BLE] failed to start '{cmd}': {ex.Message}");
            }
        }
    }
}
