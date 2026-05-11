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
            DropStaleLeLinks();
            await ForceHci0UpAsync(cancellationToken);
            await EnsureBluetoothdAsync(cancellationToken);
            await PowerOnAdapterAsync(cancellationToken);
            DisconnectStaleBluezDevices();

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

        /// <summary>
        /// Returns true if bluez has the given MAC in its device cache. Use to decide
        /// whether a fast direct connect is worth trying before falling back to a scan.
        /// </summary>
        public static bool IsDeviceCached(string macAddress)
        {
            if (string.IsNullOrWhiteSpace(macAddress))
                return false;

            (int code, string output) = Run("bluetoothctl", $"info {macAddress.Trim()}");
            // bluetoothctl exit code is 0 when device is known, non-zero ("Device not available")
            // when not cached. Also guard on the output marker to be robust across versions.
            return code == 0 && output.Contains("Device ", StringComparison.Ordinal);
        }

        /// <summary>
        /// If bluez believes the cached device is still connected from a prior session,
        /// tell it to disconnect so the next scan/connect starts from a clean slate.
        /// Returns true if a disconnect was issued.
        /// </summary>
        public static bool DisconnectCachedDevice(string macAddress)
        {
            if (string.IsNullOrWhiteSpace(macAddress))
                return false;

            (int code, string output) = Run("bluetoothctl", $"info {macAddress.Trim()}");
            if (code != 0 || !output.Contains("Connected: yes", StringComparison.Ordinal))
                return false;

            Console.WriteLine($"[BLE] bluez still has {macAddress} connected; forcing disconnect");
            Run("bluetoothctl", $"disconnect {macAddress.Trim()}");
            return true;
        }

        private static void DisconnectStaleBluezDevices()
        {
            // `bluetoothctl devices Connected` lists every device bluez still thinks
            // is connected. After a prior crash/exit this can be non-empty even when
            // hcitool con is clean, because bluez tracks its own session state.
            (int code, string output) = Run("bluetoothctl", "devices Connected");
            if (code != 0 || string.IsNullOrWhiteSpace(output))
                return;

            foreach (string line in output.Split('\n'))
            {
                string[] parts = line.Split(' ', StringSplitOptions.RemoveEmptyEntries);
                if (parts.Length < 2 || !parts[0].Equals("Device", StringComparison.Ordinal))
                    continue;

                string mac = parts[1];
                if (mac.Count(c => c == ':') != 5)
                    continue;

                Console.WriteLine($"[BLE] dropping stale bluez session for {mac}");
                Run("bluetoothctl", $"disconnect {mac}");
            }
        }

        private static void DropStaleLeLinks()
        {
            // hcitool con output:
            //   Connections:
            //       < LE 11:22:33:44:55:66 handle 64 state 1 lm SLAVE
            (int code, string output) = Run("hcitool", "con");
            if (code != 0 || string.IsNullOrWhiteSpace(output))
            {
                return;
            }

            foreach (string line in output.Split('\n'))
            {
                if (!line.Contains("LE", StringComparison.Ordinal) || !line.Contains("handle", StringComparison.Ordinal))
                {
                    continue;
                }

                string[] parts = line.Split(' ', StringSplitOptions.RemoveEmptyEntries);
                int handleIdx = Array.IndexOf(parts, "handle");
                if (handleIdx < 0 || handleIdx + 1 >= parts.Length)
                {
                    continue;
                }

                string handle = parts[handleIdx + 1];
                string mac = parts.FirstOrDefault(p => p.Count(c => c == ':') == 5) ?? "?";
                Console.WriteLine($"[BLE] dropping stale LE link {mac} (handle {handle})");
                // 0x13 = Remote User Terminated
                Run("hcitool", $"ledc {handle} 0x13");
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
