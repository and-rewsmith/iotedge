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
        static readonly Random Random = new Random();
        static readonly RetryStrategy MaxRetryStrategy = new FixedInterval(int.MaxValue, TimeSpan.FromMilliseconds(int.MaxValue));
        static readonly EndpointExecutorConfig EndpointExecutorConfig = new EndpointExecutorConfig(new TimeSpan(TimeSpan.TicksPerDay), MaxRetryStrategy, TimeSpan.FromMinutes(5));
        static readonly List<int> PossibleNumberOfClients = new List<int> {1, 2};
        static readonly List<int> PossibleFanouts = new List<int> {2, 10};
        static readonly List<int> PossibleBatchSizes = new List<int> {1, 2};
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
        static readonly int NumMessages = 16;
        static readonly List<IMessage> messagePool = CreateMessagePool();

        static List<byte[]> GetPossibleMessageBodies()
        {
            byte[] largeMessageContents = new byte[1000];
            for (int i = 0; i < largeMessageContents.Length; i++) {
               largeMessageContents[i] = (byte) i; 
            }
            return new List<byte[]> { new byte[0], new byte[] { 1, 2, 3, 4 }, largeMessageContents };
        }
        
        static List<IMessage> CreateMessagePool()
        {
            List<byte[]> possibleMessageBodies = GetPossibleMessageBodies();
            List<Dictionary<string, string>> possibleSystemPropertiesContents = new List<Dictionary<string, string>> { new Dictionary<string, string> {{ "key1", "value1" }},  new Dictionary<string, string>() }; 
            List<IMessage> messagePool = new List<IMessage>();
            string deviceId = "d1";
            for (int i = 0; i < NumMessages; i++)
            {
                if (i ==  NumMessages / 2)
                {
                    deviceId = "d2";
                }
                byte[] messagebody = possibleMessageBodies[Random.Next(possibleMessageBodies.Count)];
                Dictionary<string, string> systemPropertiesContents = possibleSystemPropertiesContents[Random.Next(possibleSystemPropertiesContents.Count)];
                messagePool.Add(
                    new Message(TelemetryMessageSource.Instance, new byte[] { 1, 2, 3, 4 }, systemPropertiesContents, new Dictionary<string, string>
                    {
                        ["connectionDeviceId"] = deviceId
                    }, 4));
            }
            return messagePool;
        }

        static Mock<ICloudProxy> CreateCloudProxyMock() 
        {
            var cloudProxy = new Mock<ICloudProxy>();
            var sequence = new MockSequence();
            double probabilityOfException = Random.NextDouble();
            for (int i = 0; i < NumMessages; i++)
            {
                if (Random.NextDouble() > probabilityOfException)
                {
                    continue;
                }
                else {
                    Exception ex = allExceptions[Random.Next(allExceptions.Count)];
                    cloudProxy.InSequence(sequence).Setup(c => c.SendMessageAsync(It.IsAny<IEdgeMessage>()))
                        .ThrowsAsync(ex);
                }
            }
            return cloudProxy;
        }

        // TODO: pick randomly from message pool
        static List<IMessage> getMessagePoolSubset(int numClients)
        {
            // if only one client then limit pool
            if (numClients == 1) 
            {
                return messagePool.GetRange(0, NumMessages / 2);
            }
            else
            {
                return messagePool;
            }
        }
        
        [Theory]
        [MemberData(nameof(GetFsmConfigurations))]
        public async Task TestEndpointExecutorFsmFuzz(int numClients, int fanout, int batchSize)
        {
            Checkpointer checkpointer = Checkpointer.CreateAsync("checkpointer", new NullCheckpointStore(0L)).Result;

            List<IMessage> messagesToSend = getMessagePoolSubset(numClients);

            Mock<ICloudProxy> cloudProxy = CreateCloudProxyMock();
            var cloudEndpoint = new CloudEndpoint(Guid.NewGuid().ToString(), _ => Task.FromResult(Option.Some(cloudProxy.Object)), new RoutingMessageConverter(), batchSize, fanout);
            IProcessor processor = cloudEndpoint.CreateProcessor();

            var machine = new EndpointExecutorFsm(cloudEndpoint, checkpointer, EndpointExecutorConfig);
            await machine.RunAsync(Commands.SendMessage(messagesToSend.ToArray()));
            // Assert.Equal(4L, checkpointer.Offset);
            Assert.NotEqual(State.DeadIdle, machine.Status.State);
        }

        public static IEnumerable<object[]> GetFsmConfigurations()
        {
            foreach (int numClients in PossibleNumberOfClients)
            {
                foreach (int fanout in PossibleFanouts)
                {
                    foreach (int batchSize in PossibleBatchSizes)
                    {
                        yield return new object[] {numClients, fanout, batchSize};
                    }
                }
            }
        }
    }
}