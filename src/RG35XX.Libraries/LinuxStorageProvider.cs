using RG35XX.Core.Interfaces;

namespace RG35XX.Libraries
{
    public class LinuxStorageProvider : IStorageProvider
    {
        public string MMC => PlatformInfo.IsRocknix ? "/storage" : "/mnt/mmc";

        public string ROOT => "/";

        public string SD => PlatformInfo.IsRocknix ? "/storage/games-external" : "/mnt/sdcard";

        public void Initialize()
        {
        }
    }
}