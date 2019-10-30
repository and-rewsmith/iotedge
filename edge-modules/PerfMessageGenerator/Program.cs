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

        public static int Main() => MainAsync().Result;

        static async Task<int> MainAsync()
        {
            Logger.LogInformation($"Starting PerfMessageGenerator with the following settings:\r\n{Settings.Current}");

            (CancellationTokenSource cts, ManualResetEventSlim completed, Option<object> handler) = ShutdownHandler.Init(TimeSpan.FromSeconds(5), Logger);

            await GenerateMessagesAsync(cts);

            completed.Set();
            handler.ForEach(h => GC.KeepAlive(h));
            Logger.LogInformation("PerfMessageGenerator Main() finished.");
            return 0;
        }

        static async Task GenerateMessagesAsync(CancellationTokenSource cts)
        {
            ModuleClient moduleClient = await ModuleUtil.CreateModuleClientAsync(
                Settings.Current.TransportType,
                ModuleUtil.DefaultTimeoutErrorDetectionStrategy,
                ModuleUtil.DefaultTransientRetryStrategy,
                Logger);

            long messageIdCounter = 0;
            string batchId = Guid.NewGuid().ToString();
            var messageBody = new byte[Settings.Current.MessageSizeInBytes];
            byte[] payload = Encoding.UTF8.GetBytes(JsonConvert.SerializeObject(messageBody));

            while (!cts.Token.IsCancellationRequested)
            {
                var message = new Message(payload);
                message.Properties.Add("sequenceNumber", messageIdCounter.ToString());
                message.Properties.Add("batchId", batchId.ToString());

                try
                {
                    await moduleClient.SendEventAsync(Settings.Current.OutputName, message);
                }
                catch (Exception e)
                {
                    Logger.LogError($"[GenerateMessagesAsync] Sequence number {messageIdCounter}, BatchId: {batchId.ToString()};{Environment.NewLine}{e}");
                }

                if (messageIdCounter % 1000 == 0)
                {
                    Logger.LogInformation($"Sent {messageIdCounter} messages");
                }

                messageIdCounter += 1;
            }
        }
    }
}
