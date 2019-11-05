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
    using Microsoft.Extensions.Logging;
    using Newtonsoft.Json;

    class Program
    {
        static readonly ILogger Logger = ModuleUtil.CreateLogger("PerfMessageGenerator");
        static long messageIdCounter = 0;

        static async Task Main()
        {
            Logger.LogInformation($"Starting perf message generator with the following settings:\r\n{Settings.Current}");

            try
            {
                string batchId = Guid.NewGuid().ToString();
                DeviceClient deviceClient = DeviceClient.CreateFromConnectionString(Settings.Current.ServiceClientConnectionString);

                using (var timers = new Timers())
                {
                    timers.Add(new TimeSpan(0,0,1), 0, () => GenerateMessagesAsync(deviceClient, batchId));

                    timers.Start();
                    (CancellationTokenSource cts, ManualResetEventSlim completed, Option<object> handler) = ShutdownHandler.Init(TimeSpan.FromSeconds(5), Logger);
                    Logger.LogInformation("Perf message generator running.");

                    await cts.Token.WhenCanceled();
                    Logger.LogInformation("Stopping timers.");
                    timers.Stop();
                    Logger.LogInformation("Closing connection to Edge Hub.");
                    await deviceClient.CloseAsync();

                    completed.Set();
                    handler.ForEach(h => GC.KeepAlive(h));
                    Logger.LogInformation("Perf message generator complete. Exiting.");
                }
            }
            catch (Exception ex)
            {
                Logger.LogError($"Error occurred during perf testing.\r\n{ex}");
            }
        }

        static async Task GenerateMessagesAsync(DeviceClient deviceClient, string batchId)
        {
            var messageBody = new byte[Settings.Current.MessageSizeInBytes];
            byte[] payload = Encoding.UTF8.GetBytes(JsonConvert.SerializeObject(messageBody));

            for (int i = 0; i < Settings.Current.MessagesPerSecond; i++)
            {
                var message = new Message(payload);
                message.Properties.Add("sequenceNumber", messageIdCounter.ToString());
                message.Properties.Add("batchId", batchId);

                try
                {
                    await deviceClient.SendEventAsync(message);
                }
                catch (Exception e)
                {
                    Logger.LogError($"[GenerateMessagesAsync] Sequence number {messageIdCounter}, BatchId: {batchId};{Environment.NewLine}{e}");
                }

                messageIdCounter += 1;
            }
        }
    }
}
