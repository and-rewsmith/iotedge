namespace Microsoft.Azure.Devices.Routing.Core.Test.Endpoints.StateMachine
{
    using System;
    using System.Collections.Generic;
    using System.Diagnostics.CodeAnalysis;
    using System.Threading.Tasks;
    using Microsoft.Azure.Devices.Edge.Util;
    using Microsoft.Azure.Devices.Edge.Util.TransientFaultHandling;
    using Microsoft.Azure.Devices.Routing.Core.Checkpointers;
    using Microsoft.Azure.Devices.Routing.Core.Endpoints;
    using Microsoft.Azure.Devices.Routing.Core.Endpoints.StateMachine;
    using Microsoft.Azure.Devices.Routing.Core.MessageSources;
    using Moq;
    using Xunit;
    using Microsoft.Azure.Devices.Client.Exceptions;
    using Microsoft.Azure.Devices.Edge.Hub.Core.Cloud;
    using Microsoft.Azure.Devices.Edge.Hub.Core.Routing;
    using Microsoft.Azure.Devices.Routing.Core;
    using IEdgeMessage = Microsoft.Azure.Devices.Edge.Hub.Core.IMessage;

    [ExcludeFromCodeCoverage]
    public class EndpointExecutorFsmFuzzTest : RoutingUnitTestBase
    {

        static readonly RetryStrategy MaxRetryStrategy = new FixedInterval(int.MaxValue, TimeSpan.FromMilliseconds(int.MaxValue));
        static readonly EndpointExecutorConfig endpointExecutorConfig = new EndpointExecutorConfig(new TimeSpan(TimeSpan.TicksPerDay), MaxRetryStrategy, TimeSpan.FromMinutes(5));
        static readonly List<int> possibleNumberOfClients = new List<int> {1, 2};
        static readonly List<int> possibleFanouts = new List<int> {2, 10};
        static readonly List<int> possibleBatchSizes = new List<int> {1, 2};
        static readonly int numMessages = 16;
        static readonly List<Exception> allExceptions = new List<Exception>
            {
                new IotHubException("Dummy"),
                new TimeoutException("Dummy"),
                new UnauthorizedException("Dummy"),
                new DeviceMaximumQueueDepthExceededException("Dummy"),
                new IotHubSuspendedException("Dummy"),
                new ArgumentException("Dummy"),
                new ArgumentNullException("Dummy")
            };
        
        // TODO: choose message size at random
        public static List<IMessage> CreateMessagePool()
        {
            List<IMessage> messagePool = new List<IMessage>();
            string deviceId = "d1";
            for (int i = 0; i < numMessages; i++)
            {
                if (i >=  numMessages / 2)
                {
                    deviceId = "d2";
                }
                messagePool.Add(
                    new Message(TelemetryMessageSource.Instance, new byte[] { 1, 2, 3, 4 }, new Dictionary<string, string> { { "key1", "value1" } }, new Dictionary<string, string>
                    {
                        ["connectionDeviceId"] = deviceId
                    }, 4));
            }
            return messagePool;
        }

        public static Mock<ICloudProxy> CreateCloudProxyMock() 
        {
            var cloudProxy = new Mock<ICloudProxy>();
            var sequence = new MockSequence();
            Random random = new Random();
            for (int i = 0; i < numMessages; i++)
            {
                Exception ex = allExceptions[random.Next(allExceptions.Count)];
                cloudProxy.InSequence(sequence).Setup(c => c.SendMessageAsync(It.IsAny<IEdgeMessage>()))
                    .ThrowsAsync(ex);
            }
            return cloudProxy;
        }
        
        [Theory]
        [MemberData(nameof(GetFsmConfigurations))]
        public async Task TestEndpointExecutorFsmFuzz(List<IMessage> messagePool, int numClients, int fanout, int batchSize)
        {
            Checkpointer checkpointer = Checkpointer.CreateAsync("checkpointer", new NullCheckpointStore(0L)).Result;

            List<IMessage> messagesToSend = messagePool;
            if (numClients == 1) {
                messagesToSend = messagePool.GetRange(0, numMessages / 2);
            } 

            Mock<ICloudProxy> cloudProxy = CreateCloudProxyMock();
            var cloudEndpoint = new CloudEndpoint(Guid.NewGuid().ToString(), _ => Task.FromResult(Option.Some(cloudProxy.Object)), new RoutingMessageConverter(), batchSize, fanout);
            IProcessor processor = cloudEndpoint.CreateProcessor();

            var machine = new EndpointExecutorFsm(cloudEndpoint, checkpointer, endpointExecutorConfig);
            await machine.RunAsync(Commands.SendMessage(messagesToSend.ToArray()));
            // Assert.Equal(4L, checkpointer.Offset);
            Assert.NotEqual(State.DeadIdle, machine.Status.State);
        }

        public static IEnumerable<object[]> GetFsmConfigurations()
        {
            List<IMessage> messagePool = CreateMessagePool();
            for (int c = 0; c < possibleNumberOfClients.Count; c++)
            {
                for (int f = 0; f < possibleFanouts.Count; f++)
                {
                    for (int b = 0; b < possibleBatchSizes.Count; b++)
                    {
                        yield return new object[] {messagePool, possibleNumberOfClients[c], possibleFanouts[f], possibleBatchSizes[b]};
                    }
                }
            }
        }
    }
}