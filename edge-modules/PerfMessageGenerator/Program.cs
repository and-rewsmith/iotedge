// Copyright (c) Microsoft. All rights reserved.
namespace PerfMessageGenerator
{
    using System;
    using System.Text;
    using System.Threading;
    using System.Threading.Tasks;
    using Microsoft.Azure.Devices.Client;
    using Microsoft.Azure.Devices.Edge.ModuleUtil;
    using Microsoft.Azure.Devices.Edge.Util;
    using Microsoft.Azure.Devices.Shared;
    using Microsoft.Extensions.Logging;
    using Newtonsoft.Json;

    class Program
    {
        static readonly ILogger Logger = ModuleUtil.CreateLogger("LoadGen");

        static long messageIdCounter = 0;

        static async Task Main()
        {
            Logger.LogInformation($"Starting perf message generator with the following settings:\r\n{Settings.Current}");

            try
            {
                ModuleClient moduleClient = await ModuleUtil.CreateModuleClientAsync(
                    Settings.Current.TransportType,
                    ModuleUtil.DefaultTimeoutErrorDetectionStrategy,
                    ModuleUtil.DefaultTransientRetryStrategy,
                    Logger);

                using (var timers = new Timers())
                {
                    Guid batchId = Guid.NewGuid();
                    Logger.LogInformation($"Batch Id={batchId}");

                    // setup the message timer
                    timers.Add(
                        Settings.Current.MessageFrequency,
                        0,
                        () => GenerateMessageAsync(moduleClient, batchId));

                    timers.Start();
                    (CancellationTokenSource cts, ManualResetEventSlim completed, Option<object> handler) = ShutdownHandler.Init(TimeSpan.FromSeconds(5), Logger);
                    Logger.LogInformation("Perf message generator running.");

                    await cts.Token.WhenCanceled();
                    Logger.LogInformation("Stopping timers.");
                    timers.Stop();
                    Logger.LogInformation("Closing connection to Edge Hub.");
                    await moduleClient.CloseAsync();

                    completed.Set();
                    handler.ForEach(h => GC.KeepAlive(h));
                    Logger.LogInformation("Perf message generator complete. Exiting.");
                }
            }
            catch (Exception ex)
            {
                Logger.LogError($"Error occurred during perf message generator gen.\r\n{ex}");
            }
        }

        static async Task GenerateMessageAsync(ModuleClient client, Guid batchId)
        {
            try
            {
                int batchSize = 100;
                var messageBody = new byte[Settings.Current.MessageSizeInBytes];
                byte[] payload = Encoding.UTF8.GetBytes(JsonConvert.SerializeObject(messageBody));
                for (int i = 0; i < batchSize; i++)
                {
                    var message = new Message(payload);
                    message.Properties.Add("sequenceNumber", messageIdCounter.ToString());
                    message.Properties.Add("batchId", batchId.ToString());
                    await client.SendEventAsync(Settings.Current.OutputName, message);
                }

                if (messageIdCounter % 1000 == 0)
                {
                    Logger.LogDebug($"Sent {messageIdCounter} messages");
                }
            }
            catch (Exception e)
            {
                Logger.LogError($"[GenerateMessageAsync] Sequence number {messageIdCounter}, BatchId: {batchId.ToString()};{Environment.NewLine}{e}");
            }
        }
    }
}
