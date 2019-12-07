// Copyright (c) Microsoft. All rights reserved.
namespace Microsoft.Azure.Devices.Edge.Util.Metrics.Prometheus.Net
{
    using System;
    using System.Collections.Generic;
    using System.Globalization;
    using System.IO;
    using System.Threading;
    using System.Threading.Tasks;
    using global::Prometheus;
    using Microsoft.Extensions.Logging;

    public class MetricsProvider : IMetricsProvider
    {
        const string CounterNameFormat = "{0}_{1}_total";
        const string NameFormat = "{0}_{1}";
        const string InstanceFileName = "instance";
        const string InstanceFolderName = "metrics_instance";
        readonly string instanceFileAbsolutePath;
        readonly string namePrefix;
        readonly List<string> defaultLabelNames;
        readonly ILogger logger = Logger.Factory.CreateLogger<MetricsProvider>();

        public MetricsProvider(string namePrefix, string iotHubName, string deviceId, string storagePath)
        {
            this.namePrefix = Preconditions.CheckNonWhiteSpace(namePrefix, nameof(namePrefix));

            Preconditions.CheckNonWhiteSpace(iotHubName, nameof(iotHubName));
            Preconditions.CheckNonWhiteSpace(deviceId, nameof(deviceId));
            Preconditions.CheckNonWhiteSpace(storagePath, nameof(storagePath));

            string storageDirectory = Path.Combine(storagePath, InstanceFolderName);
            string instanceNumber;
            try
            {
                Directory.CreateDirectory(storageDirectory);
                this.instanceFileAbsolutePath = Path.Combine(storageDirectory, InstanceFileName);
                instanceNumber = this.GetInstanceNumber();
            }
            catch (Exception ex)
            {
                this.logger.LogError(ex, "Could not create metrics directory. Metrics instance number not supported.");
                instanceNumber = "0";
            }

            this.defaultLabelNames = new List<string> { iotHubName, deviceId, instanceNumber };

            // TODO:
            // By default, the Prometheus.Net library emits some default metrics.
            // While useful, these are emitted without any tags. This will make it hard to
            // consume and make sense of these metrics. So suppressing the default metrics for
            // now. We can look at ways to add tags to the default metrics, or emiting the
            // metrics manually.
            Metrics.SuppressDefaultMetrics();
        }

        public IMetricsGauge CreateGauge(string name, string description, List<string> labelNames)
            => new MetricsGauge(this.GetName(name), description, this.GetLabelNames(labelNames), this.defaultLabelNames);

        public IMetricsCounter CreateCounter(string name, string description, List<string> labelNames)
            => new MetricsCounter(this.GetCounterName(name), description, this.GetLabelNames(labelNames), this.defaultLabelNames);

        public IMetricsTimer CreateTimer(string name, string description, List<string> labelNames)
            => new MetricsTimer(this.GetName(name), description, this.GetLabelNames(labelNames), this.defaultLabelNames);

        public IMetricsHistogram CreateHistogram(string name, string description, List<string> labelNames)
            => new MetricsHistogram(this.GetName(name), description, this.GetLabelNames(labelNames), this.defaultLabelNames);

        public IMetricsDuration CreateDuration(string name, string description, List<string> labelNames)
            => new MetricsDuration(this.GetName(name), description, this.GetLabelNames(labelNames), this.defaultLabelNames);

        public async Task<byte[]> GetSnapshot(CancellationToken cancellationToken)
        {
            using (var ms = new MemoryStream())
            {
                await Metrics.DefaultRegistry.CollectAndExportAsTextAsync(ms, cancellationToken);
                return ms.ToArray();
            }
        }

        internal CollectorRegistry DefaultRegistry => Metrics.DefaultRegistry;

        string GetCounterName(string name) => string.Format(CultureInfo.InvariantCulture, CounterNameFormat, this.namePrefix, name);

        string GetName(string name) => string.Format(CultureInfo.InvariantCulture, NameFormat, this.namePrefix, name);

        List<string> GetLabelNames(List<string> labelNames)
        {
            var allLabelNames = new List<string>
            {
                MetricsConstants.IotHubLabel,
                MetricsConstants.DeviceIdLabel,
                "instance_number"
            };
            allLabelNames.AddRange(labelNames);
            return allLabelNames;
        }

        /// <summary>
        /// Reads or creates a file to determine how many times this module has been restarted.
        /// This is because prometheous metrics are cleared on restart.
        /// Includung an instance number makes it clear when a counter reset.
        /// If the time series of metrics goes
        ///     {instance=1) 500
        ///     {instance=1) 600
        ///     {instance=1) 700
        ///     {instance=2) 90
        ///     {instance=2) 190
        /// it is clear what happened.
        /// </summary>
        /// <returns>Instance number.</returns>
        string GetInstanceNumber()
        {
            try
            {
                if (!File.Exists(this.instanceFileAbsolutePath))
                {
                    File.WriteAllText(this.instanceFileAbsolutePath, "1");
                    return "1";
                }

                string string_num = File.ReadAllText(this.instanceFileAbsolutePath);
                string_num = (int.Parse(string_num) + 1).ToString();
                File.WriteAllText(this.instanceFileAbsolutePath, string_num);
                return string_num;
            }
            catch (Exception ex)
            {
                Console.WriteLine($"{DateTime.UtcNow.ToLogString()} Failed to access metrics instance file:\n{ex}");
                return "0";
            }
        }
    }
}
