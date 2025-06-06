using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading.Tasks;

namespace RG35XX.Libraries
{
    public partial class LinuxGamePadReader
    {
        [StructLayout(LayoutKind.Sequential)]
        private struct JoystickEvent
        {
            public uint Time;     // Event timestamp in milliseconds

            public short Value;   // Value

            public byte Type;     // Event type

            public byte Number;   // Axis/button number
        }
    }
}
