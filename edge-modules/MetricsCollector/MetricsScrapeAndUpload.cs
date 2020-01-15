// Copyright (c) Microsoft. All rights reserved.
namespace MetricsCollector
{
    using System;
    using System.Collections.Generic;
    using System.Threading;
    using System.Threading.Tasks;
    using Microsoft.Azure.Devices.Edge.Agent.Diagnostics;
    using Microsoft.Azure.Devices.Edge.Agent.Diagnostics.Publisher;
    using Microsoft.Azure.Devices.Edge.Util;
    using Microsoft.Extensions.Logging;
    using ILogger = Microsoft.Extensions.Logging.ILogger;

    class MetricsScrapeAndUpload : IDisposable
    {
        static readonly ILogger Logger = MetricsUtil.CreateLogger("MetricsCollector");
        readonly IMetricsScraper scraper;
        readonly IMetricsPublisher publisher;
        readonly Dictionary<string, string> testSpecificTags;
        readonly Guid metricsCollectorRuntimeId;
        PeriodicTask periodicScrapeAndUpload;

        public MetricsScrapeAndUpload(IMetricsScraper scraper, IMetricsPublisher publisher, Dictionary<string, string> testSpecificTags, Guid metricsCollectorRuntimeId)
        {
            this.scraper = Preconditions.CheckNotNull(scraper);
            this.publisher = Preconditions.CheckNotNull(publisher);
            this.testSpecificTags = Preconditions.CheckNotNull(testSpecificTags);
            Preconditions.CheckArgument(metricsCollectorRuntimeId != Guid.Empty);
            this.metricsCollectorRuntimeId = metricsCollectorRuntimeId;
        }

        public void Start(TimeSpan scrapeAndUploadInterval)
        {
            this.periodicScrapeAndUpload = new PeriodicTask(this.ScrapeAndUploadMetricsAsync, scrapeAndUploadInterval, scrapeAndUploadInterval, Logger, "Scrape and Upload Metrics");
        }

        public void Dispose()
        {
           this.periodicScrapeAndUpload?.Dispose();
        }

        async Task ScrapeAndUploadMetricsAsync(CancellationToken cancellationToken)
        {
            try
            {
                IEnumerable<Metric> metrics = await this.scraper.ScrapeEndpointsAsync(cancellationToken);
                await this.publisher.PublishAsync(this.GetGuidTaggedMetrics(metrics), cancellationToken);
                Logger.LogInformation("Successfully scraped and uploaded metrics");
            }
            catch (Exception e)
            {
                Logger.LogError(e, "Error scraping and uploading metrics");
            }
        }

        IEnumerable<Metric> GetGuidTaggedMetrics(IEnumerable<Metric> metrics)
        {
            foreach (Metric metric in metrics)
            {
                Dictionary<string, string> metricTags = new Dictionary<string, string>(metric.Tags);

                metricTags.Add("testId", this.metricsCollectorRuntimeId.ToString());
                metricTags.Add("testType", Settings.Current.TestType);
                foreach (KeyValuePair<string, string> pair in this.testSpecificTags)
                {
                    metricTags[pair.Key] = pair.Value;
                }

                yield return new Metric(metric.TimeGeneratedUtc, metric.Name, metric.Value, metricTags);
            }
        }
    }
}
