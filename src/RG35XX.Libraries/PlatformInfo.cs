namespace RG35XX.Libraries;

/// <summary>The OS the app runs on, for the few things that differ per OS.</summary>
public static class PlatformInfo
{
	/// <summary>True on ROCKNIX, from OS_NAME in /etc/os-release.</summary>
	public static bool IsRocknix { get; } = ReadIsRocknix();

	private static bool ReadIsRocknix()
	{
		try
		{
			return File.ReadLines("/etc/os-release")
				.Any(line => line.Replace("\"", "").Trim() == "OS_NAME=ROCKNIX");
		}
		catch (IOException)
		{
			return false;
		}
		catch (UnauthorizedAccessException)
		{
			return false;
		}
	}
}
