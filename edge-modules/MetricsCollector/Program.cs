namespace MetricsCollector
{
    using System;
    using System.Linq;
    using System.Runtime.Loader;
    using System.Threading;
    using System.Threading.Tasks;
    using Microsoft.Azure.Devices.Client;
    using Microsoft.Azure.Devices.Client.Transport.Mqtt;
    using Microsoft.Azure.Devices.Edge.ModuleUtil;
    using Microsoft.Azure.Devices.Edge.Util;
    using Microsoft.Extensions.Logging;
    using Newtonsoft.Json;

    internal class Program
    {
        static readonly Version ExpectedSchemaVersion = new Version("1.0");
        static Timer ScrapingTimer;
        static readonly ILogger Logger = ModuleUtil.CreateLogger("MetricsCollector");

        public static int Main() => MainAsync().Result;

        private static async Task<int> MainAsync()
        {
            Logger.LogInformation($"Starting metrics collector with the following settings:\r\n{Settings.Current}");

            await Init();

            (CancellationTokenSource cts, ManualResetEventSlim completed, Option<object> handler) = ShutdownHandler.Init(TimeSpan.FromSeconds(5), Logger);

            // Wait until the app unloads or is cancelled
            AssemblyLoadContext.Default.Unloading += ctx => cts.Cancel();
            Console.CancelKeyPress += (sender, cpe) => cts.Cancel();
            await WhenCancelled(cts.Token, completed, handler);
            Logger.LogInformation("MetricsCollector Main() finished.");
            return 0;
        }

        /// <summary>
        ///     Handles cleanup operations when app is cancelled or unloads
        /// </summary>
        public static Task WhenCancelled(CancellationToken cancellationToken, ManualResetEventSlim completed, Option<object> handler)
        {
            var tcs = new TaskCompletionSource<bool>();
            cancellationToken.Register(
                s => { 
                    completed.Set();
                    handler.ForEach(h => GC.KeepAlive(h));
                    ((TaskCompletionSource<bool>)s).SetResult(true);
                },
                tcs);
            return tcs.Task;
        }

        /// <summary>
        ///     Initializes the ModuleClient and sets up the callback to receive
        ///     messages containing temperature information
        /// </summary>
        private static async Task Init()
        {
            var mqttSetting = new MqttTransportSettings(TransportType.Mqtt_Tcp_Only);
            ITransportSettings[] settings = { mqttSetting };

            // Open a connection to the Edge runtime
            var ioTHubModuleClient = await ModuleClient.CreateFromEnvironmentAsync(settings);
            await ioTHubModuleClient.OpenAsync();
            Logger.LogInformation("IoT Hub module client initialized.");

            Configuration configuration = await GetConfiguration(ioTHubModuleClient);
            Logger.LogInformation($"Obtained configuration: {configuration}");

            var messageFormatter = new MessageFormatter(configuration.MetricsFormat, Settings.Current.MessageIdentifier);
            var scraper = new Scraper(configuration.Endpoints.Values.ToList());

            IMetricsSync metricsSync;
            if (configuration.SyncTarget == SyncTarget.AzureLogAnalytics)
            {
                string workspaceId = Settings.Current.AzMonWorkspaceId;
                string workspaceKey = Settings.Current.AzMonWorkspaceKey;
                string customLogName = Settings.Current.AzMonCustomLogName;
                metricsSync = new LogAnalyticsMetricsSync(messageFormatter, scraper, new AzureLogAnalytics(workspaceId, workspaceKey, customLogName));
            }
            else
            {
                metricsSync = new IoTHubMetricsSync(messageFormatter, scraper, ioTHubModuleClient);
            }
            
            var scrapingInterval = TimeSpan.FromSeconds(configuration.ScrapeFrequencySecs);
            ScrapingTimer = new Timer(ScrapeAndSync, metricsSync, scrapingInterval, scrapingInterval);
        }

        private static async void ScrapeAndSync(object context)
        {
            try
            {
                var metricsSync = (IMetricsSync)context;
                await metricsSync.ScrapeAndSync();
            }
            catch (Exception e)
            {
                Logger.LogError($"Error scraping and syncing metrics to IoTHub - {e}");
            }
        }

        private static async Task<Configuration> GetConfiguration(ModuleClient ioTHubModuleClient)
        {
            var twin = await ioTHubModuleClient.GetTwinAsync();
            var desiredPropertiesJson = twin.Properties.Desired.ToJson();
            var configuration = JsonConvert.DeserializeObject<Configuration>(desiredPropertiesJson);
            if (ExpectedSchemaVersion.CompareMajorVersion(configuration.SchemaVersion, "logs upload request schema") !=
                0)
                throw new InvalidOperationException($"Payload schema version is not valid - {desiredPropertiesJson}");
            return configuration;
        }
    }
}