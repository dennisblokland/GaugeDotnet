namespace RG35XX.Libraries;

/// <summary>
/// Prevents MuOS screen dimming by writing idle_inhibit = 1 (INHIBIT_BOTH).
/// MuOS idle.sh resets this every 5s, so callers should poke every ~3s.
/// </summary>
public static class ScreenKeepAlive
{
	private const string InhibitPath = "/opt/muos/config/system/idle_inhibit";
	private static bool _available;
	private static bool _initialized;

	/// <summary>
	/// Write idle inhibit to prevent MuOS dimming/sleep. No-ops on non-MuOS systems.
	/// </summary>
	public static void Poke()
	{
		if (!_initialized)
		{
			_initialized = true;
			_available = Directory.Exists("/opt/muos/config/system");
		}

		if (!_available) return;

		try
		{
			File.WriteAllText(InhibitPath, "1");
		}
		catch
		{
			_available = false;
		}
	}
}
