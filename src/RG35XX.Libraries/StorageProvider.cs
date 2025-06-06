using RG35XX.Core.Interfaces;

namespace RG35XX.Libraries
{
    public class StorageProvider : IStorageProvider
    {
        private readonly IStorageProvider _storageProvider;

        public string MMC => _storageProvider.MMC;

        public string SD => _storageProvider.SD;

        public string ROOT => _storageProvider.ROOT;

        public StorageProvider()
        {
            _storageProvider = new RG35XX.Libraries.LinuxStorageProvider();
        }
        
        public void Initialize()
        {
            _storageProvider.Initialize();
        }

        public bool IsSDMounted()
        {
            if (!Directory.Exists(SD))
            {
                return false;
            }

            return Directory.EnumerateDirectories(SD).Any();
        }
    }
}