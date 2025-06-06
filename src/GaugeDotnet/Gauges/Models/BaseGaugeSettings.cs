using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace GaugeDotnet.Gauges.Models
{
    public class BaseGaugeSettings
    {
        public decimal InitialValue { get; set; } = 0;
        public decimal MinValue { get; set; } = 0;
        public decimal MaxValue { get; set; } = 100;
        public string Unit { get; set; } = "";
        public string Title { get; set; } = "";
    }
}
