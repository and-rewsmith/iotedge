// Copyright (c) Microsoft. All rights reserved.
namespace MessagesAnalyzer
{
    using System;
    using System.Collections.Generic;
    using System.Linq;
    using System.Runtime.Loader;
    using System.Threading;
    using System.Threading.Tasks;
    using Microsoft.AspNetCore;
    using Microsoft.AspNetCore.Hosting;
    using Microsoft.Azure.Devices.Common;
    using Microsoft.Azure.Devices.Edge.Util;
    using Microsoft.Azure.EventHubs;
    using Microsoft.Extensions.Logging;

    class Program
    {
        static readonly ILogger Log = Logger.Factory.CreateLogger<Program>();

        static async Task Main(string[] args)
        {
            Log.LogInformation($"Starting analyzer for [deviceId: {Settings.Current.DeviceId}] with [consumerGroupId: {Settings.Current.ConsumerGroupId}], exclude-modules: {string.Join(", ", Settings.Current.ExcludedModuleIds.ToArray())}");

            await ReceiveMessages();

            var cts = new CancellationTokenSource();
            AssemblyLoadContext.Default.Unloading += (ctx) => cts.Cancel();
            Console.CancelKeyPress += (sender, cpe) => cts.Cancel();
            var tcs = new TaskCompletionSource<bool>();
            cts.Token.Register(s => ((TaskCompletionSource<bool>)s).SetResult(true), tcs);

            await CreateWebHostBuilder(args).Build().RunAsync(cts.Token);
        }

        static IWebHostBuilder CreateWebHostBuilder(string[] args) =>
            WebHost.CreateDefaultBuilder(args)
                .UseUrls($"http://*:{Settings.Current.WebhostPort}")
                .UseStartup<Startup>();

        // TODO: make work for module use case
        static async Task ReceiveMessages()
        {
            var builder = new EventHubsConnectionStringBuilder(Settings.Current.EventHubConnectionString);
            Log.LogInformation($"Receiving events from device '{Settings.Current.DeviceId}' on Event Hub '{builder.EntityPath}'");

            EventHubClient eventHubClient =
                EventHubClient.CreateFromConnectionString(builder.ToString());

            string consumerGroupId = Settings.Current.ConsumerGroupId;
            EventPosition eventPosition = EventPosition.FromEnqueuedTime(DateTime.UtcNow);

            string[] deviceNames = { "perf-device-0", "perf-device-1", "perf-device-2", "perf-device-3", "perf-device-4", "perf-device-5", "perf-device-6", "perf-device-7", "perf-device-8", "perf-device-9", "perf-device-10" };
            foreach (string deviceName in deviceNames)
            {
                PartitionReceiver eventHubReceiver = eventHubClient.CreateReceiver(
                    deviceName,
                    EventHubPartitionKeyResolver.ResolveToPartition(deviceName, (await eventHubClient.GetRuntimeInformationAsync()).PartitionCount),
                    EventPosition.FromEnqueuedTime(DateTime.UtcNow));

                eventHubReceiver.SetReceiveHandler(new PartitionReceiveHandler(deviceName, Settings.Current.ExcludedModuleIds));
            }
        }
    }
}
