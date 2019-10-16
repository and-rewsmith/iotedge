// Copyright (c) Microsoft. All rights reserved.
namespace Microsoft.Azure.Devices.Routing.Core.Test.Endpoints
{
    using System;
    using System.Collections.Concurrent;
    using System.Collections.Generic;
    using System.Threading.Tasks;
    using Microsoft.Azure.Devices.Edge.Util.Concurrency;
    public class TestMessageStore : IMessageStore
    {
        readonly ConcurrentDictionary<string, TestMessageQueue> endpointQueues = new ConcurrentDictionary<string, TestMessageQueue>();

        public void Dispose()
        {
        }

        public Task<long> Add(string endpointId, IMessage message)
        {
            TestMessageQueue queue = this.GetQueue(endpointId);
            return queue.Add(message);
        }

        public IMessageIterator GetMessageIterator(string endpointId, long startingOffset) => this.GetQueue(endpointId);

        public IMessageIterator GetMessageIterator(string endpointId) => this.GetQueue(endpointId);

        public Task AddEndpoint(string endpointId)
        {
            this.endpointQueues[endpointId] = new TestMessageQueue();
            return Task.CompletedTask;
        }

        public Task RemoveEndpoint(string endpointId)
        {
            this.endpointQueues.Remove(endpointId, out TestMessageQueue _);
            return Task.CompletedTask;
        }

        public void SetTimeToLive(TimeSpan timeToLive) => throw new NotImplementedException();

        public List<IMessage> GetReceivedMessagesForEndpoint(string endpointId) => this.GetQueue(endpointId).Queue;

        TestMessageQueue GetQueue(string endpointId) => this.endpointQueues.GetOrAdd(endpointId, new TestMessageQueue());

        class TestMessageQueue : IMessageIterator
        {
            readonly List<IMessage> queue = new List<IMessage>();
            readonly AsyncLock queueLock = new AsyncLock();
            int index;

            public List<IMessage> Queue => this.queue;

            public async Task<long> Add(IMessage message)
            {
                using (await this.queueLock.LockAsync())
                {
                    this.queue.Add(message);
                    return (long)this.queue.Count - 1;
                }
            }

            public async Task<IEnumerable<IMessage>> GetNext(int batchSize)
            {
                using (await this.queueLock.LockAsync())
                {
                    var batch = new List<IMessage>();
                    for (int i = 0; i < batchSize && this.index < this.queue.Count; i++, this.index++)
                    {
                        batch.Add(this.queue[this.index]);
                    }

                    return batch;
                }
            }
        }
    }
}