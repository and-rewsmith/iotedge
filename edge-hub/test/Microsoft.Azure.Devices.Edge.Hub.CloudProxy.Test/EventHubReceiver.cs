// Copyright (c) Microsoft. All rights reserved.
namespace Microsoft.Azure.Devices.Edge.Hub.CloudProxy.Test
{
    using System;
    using System.Collections.Generic;
    using System.Threading.Tasks;
    using Microsoft.Azure.Devices.Common;
    using Microsoft.Azure.EventHubs;

    public class EventHubReceiver
    {
        readonly EventHubClient eventHubClient;

        public EventHubReceiver(string eventHubConnectionString)
        {
            this.eventHubClient = EventHubClient.CreateFromConnectionString(eventHubConnectionString);
        }

        public async Task<IList<EventData>> GetMessagesForDevice(string deviceId, DateTime startTime, int maxPerPartition = 10, int waitTimeSecs = 5)
        {
            var messages = new List<EventData>();

            PartitionReceiver partitionReceiver = this.eventHubClient.CreateReceiver(
                PartitionReceiver.DefaultConsumerGroupName,
                EventHubPartitionKeyResolver.ResolveToPartition(deviceId, (await this.eventHubClient.GetRuntimeInformationAsync()).PartitionCount),
                EventPosition.FromEnqueuedTime(startTime));

            // Retry a few times to make sure we get all expected messages.
            for (int i = 0; i < 3; i++)
            {
                IEnumerable<EventData> events = await partitionReceiver.ReceiveAsync(maxPerPartition, TimeSpan.FromSeconds(waitTimeSecs));
                if (events != null)
                {
                    messages.AddRange(events);
                }

                if (i < 3)
                {
                    await Task.Delay(TimeSpan.FromSeconds(5));
                }
            }

            await partitionReceiver.CloseAsync();

            return messages;
        }

        public async Task Close()
        {
            await this.eventHubClient.CloseAsync();
        }
    }
}
