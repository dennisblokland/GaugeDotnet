using GaugeDotnet.Configuration;
using GaugeDotnet.Gauges.Models;

namespace GaugeDotnet.Gauges
{
    public static class GaugeFactory
    {
        public static List<(BaseGauge Gauge, string DataSource)> BuildScreens(AppConfig config, int screenWidth, int screenHeight)
        {
            List<(BaseGauge Gauge, string DataSource)> screens = new();
            foreach (ScreenConfig screenConfig in config.Screens)
            {
                GaugeConfig gaugeConfig = screenConfig.Gauge;
                BaseGauge gauge = CreateGauge(gaugeConfig, screenWidth, screenHeight);

                if (gaugeConfig.Type != GaugeType.Grid)
                {
                    gauge.SetColorHex(gaugeConfig.ColorHex);
                }

                screens.Add((gauge, gaugeConfig.DataSource));
            }
            return screens;
        }

        private static BaseGauge CreateGauge(GaugeConfig config, int screenWidth, int screenHeight)
        {
            return config.Type switch
            {
                GaugeType.Circular => new CircularGauge(
                    new CircularGaugeSettings
                    {
                        InitialValue = config.InitialValue,
                        MinValue = config.MinValue,
                        MaxValue = config.MaxValue,
                        Unit = config.Unit,
                        Title = config.Title,
                        Decimals = config.Decimals,
                        SegmentCount = config.SegmentCount,
                        Smoothing = config.Smoothing,
                    }, screenWidth, screenHeight),

                GaugeType.Histogram => new HistogramGauge(
                    new HistogramGaugeSettings
                    {
                        InitialValue = config.InitialValue,
                        MinValue = config.MinValue,
                        MaxValue = config.MaxValue,
                        Unit = config.Unit,
                        Title = config.Title,
                        Decimals = config.Decimals,
                        MaxDataPoints = config.MaxDataPoints,
                        IntervalMs = config.IntervalMs,
                    }, screenWidth, screenHeight),

                GaugeType.Needle => new NeedleGauge(
                    new NeedleGaugeSettings
                    {
                        InitialValue = config.InitialValue,
                        MinValue = config.MinValue,
                        MaxValue = config.MaxValue,
                        Unit = config.Unit,
                        Title = config.Title,
                        Decimals = config.Decimals,
                        Smoothing = config.Smoothing,
                    }, screenWidth, screenHeight),

                GaugeType.Digital => new DigitalGauge(
                    new DigitalGaugeSettings
                    {
                        InitialValue = config.InitialValue,
                        MinValue = config.MinValue,
                        MaxValue = config.MaxValue,
                        Unit = config.Unit,
                        Title = config.Title,
                        Decimals = config.Decimals,
                    }, screenWidth, screenHeight),

                GaugeType.Sweep => new SweepGauge(
                    new SweepGaugeSettings
                    {
                        InitialValue = config.InitialValue,
                        MinValue = config.MinValue,
                        MaxValue = config.MaxValue,
                        Unit = config.Unit,
                        Title = config.Title,
                        Decimals = config.Decimals,
                        Smoothing = config.Smoothing,
                    }, screenWidth, screenHeight),

                GaugeType.MinMax => new MinMaxGauge(
                    new MinMaxGaugeSettings
                    {
                        InitialValue = config.InitialValue,
                        MinValue = config.MinValue,
                        MaxValue = config.MaxValue,
                        Unit = config.Unit,
                        Title = config.Title,
                        Decimals = config.Decimals,
                        SegmentCount = config.SegmentCount,
                        Smoothing = config.Smoothing,
                    }, screenWidth, screenHeight),

                GaugeType.Grid => new GridGauge(
                    new GridGaugeSettings
                    {
                        InitialValue = config.InitialValue,
                        MinValue = config.MinValue,
                        MaxValue = config.MaxValue,
                        Unit = config.Unit,
                        Title = config.Title,
                        Cells = config.Cells,
                    }, screenWidth, screenHeight),

                _ => new BarGauge(
                    new BarGaugeSettings
                    {
                        InitialValue = config.InitialValue,
                        MinValue = config.MinValue,
                        MaxValue = config.MaxValue,
                        Unit = config.Unit,
                        Title = config.Title,
                        Decimals = config.Decimals,
                        SegmentCount = config.SegmentCount,
                        Smoothing = config.Smoothing,
                    }, screenWidth, screenHeight),
            };
        }

        public static void UpdateGaugeValues(BaseGauge gauge, string dataSource, IMeDevice device)
        {
            if (gauge is GridGauge gridGauge)
            {
                for (int i = 0; i < gridGauge.CellCount; i++)
                {
                    GridCellConfig cell = gridGauge.GetCellConfig(i);
                    float cellValue = DataSourceMapper.ReadValue(device.Data, cell.DataSource);
                    gridGauge.SetCellValue(i, cellValue);
                }
            }
            else
            {
                float value = DataSourceMapper.ReadValue(device.Data, dataSource);
                gauge.SetValue(value);
            }
        }
    }
}
