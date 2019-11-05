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
            await Task.Delay(1000 * 60 * 5);
            return 0;
        }

        static async Task GenerateMessagesAsync(CancellationTokenSource cts)
        {
            DeviceClient deviceClient;
            try
            {
                deviceClient = DeviceClient.CreateFromConnectionString(Settings.Current.ServiceClientConnectionString);
            }
            catch (Exception e)
            {
                Logger.LogError("Failed to initialize device client");
                return;
            }

            long messageIdCounter = 0;
            string batchId = Guid.NewGuid().ToString();
            var messageBody = new byte[Settings.Current.MessageSizeInBytes];
            byte[] payload = Encoding.UTF8.GetBytes(JsonConvert.SerializeObject(messageBody));
            while (!cts.Token.IsCancellationRequested)
            {
                if (messageIdCounter % 1000 == 0)
                {
                    Logger.LogInformation($"{batchId}: Sent {messageIdCounter} messages");
                }

                var message = new Message(payload);
                message.Properties.Add("sequenceNumber", messageIdCounter.ToString());
                message.Properties.Add("batchId", batchId);

                try
                {
                    Message message = new Message();
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
