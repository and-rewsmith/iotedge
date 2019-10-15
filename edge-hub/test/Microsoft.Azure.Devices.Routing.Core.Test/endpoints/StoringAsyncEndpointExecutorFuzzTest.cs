namespace Microsoft.Azure.Devices.Routing.Core.Test.Endpoints
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
    using Xunit.Abstractions;
    using Microsoft.Azure.Devices.Client.Exceptions;
    using Microsoft.Azure.Devices.Edge.Hub.Core.Cloud;
    using Microsoft.Azure.Devices.Edge.Hub.Core.Routing;
    using Microsoft.Azure.Devices.Routing.Core;
    using IEdgeMessage = Microsoft.Azure.Devices.Edge.Hub.Core.IMessage;
    using Microsoft.Azure.Devices.Routing.Core.Test.Checkpointers;

    [ExcludeFromCodeCoverage]
    public class StoringAsyncEndpointExecutorFuzzTest : RoutingUnitTestBase
    {
        static readonly Random Random = new Random();
        static readonly RetryStrategy MaxRetryStrategy = new FixedInterval(int.MaxValue, TimeSpan.FromMilliseconds(1));
        static readonly EndpointExecutorConfig EndpointExecutorConfig = new EndpointExecutorConfig(new TimeSpan(TimeSpan.TicksPerDay), MaxRetryStrategy, TimeSpan.FromMinutes(5));
        static readonly List<int> PossibleNumberOfClients = new List<int> {1, 2};
        static readonly List<int> PossibleFanouts = new List<int> {2, 10};
        static readonly List<int> PossibleBatchSizes = new List<int> {1, 2};
        static readonly List<byte[]> PossibleMessageBodies = GetPossibleMessageBodies();
        // TODO: consider generating these dynamically
        static readonly List<Dictionary<string, string>> PossibleMessagePropertiesContents = new List<Dictionary<string, string>> { new Dictionary<string, string> {{ "key1", "value1" }},  new Dictionary<string, string>() }; 
        static readonly List<Exception> AllExceptions = new List<Exception>
            {
                new IotHubException("Dummy"),
                new TimeoutException("Dummy"),
                new UnauthorizedException("Dummy"),
                new DeviceMaximumQueueDepthExceededException("Dummy"),
                new IotHubSuspendedException("Dummy"),
                // new ArgumentException("Dummy"),
                // new ArgumentNullException("Dummy"),
                // new DeviceAlreadyExistsException("Dummy"),
                // new DeviceDisabledException("Dummy"),
                // new DeviceMessageLockLostException("Dummy"),
                // new IotHubCommunicationException("Dummy"),
                // new IotHubNotFoundException("Dummy"),
                // new IotHubThrottledException("Dummy"),
                // new MessageTooLargeException("Dummy"),
                // new QuotaExceededException("Dummy"),
                // new ServerBusyException("Dummy"),
                // new ServerErrorException("Dummy"),
            };
        static readonly int MessagesPerClient = 8; // must be at least 8 to exercise all code paths in the FSM surrounding message send
        static readonly string ClientIdentityPlaceholder = "connectionDeviceId";
        static readonly string MessageOrderingPlaceholder = "msgSequenceNumber";
        static readonly string ExceptionIndexPlaceholder = "exceptionIndex";
        private ITestOutputHelper outputHelper;

        public StoringAsyncEndpointExecutorFuzzTest(ITestOutputHelper outputHelper)
        {
            this.outputHelper = outputHelper;
        }

        static List<byte[]> GetPossibleMessageBodies()
        {
            byte[] largeMessageContents = new byte[1000];
            for (int i = 0; i < largeMessageContents.Length; i++) {
               largeMessageContents[i] = (byte) i; 
            }
            return new List<byte[]> { new byte[0], new byte[] { 1, 2, 3, 4 }, largeMessageContents };
        }

        List<IMessage> CreateMessagePool(int numClients)
        {
            List<IMessage> messagePool = new List<IMessage>();
            int numMessages = numClients * MessagesPerClient;
            int deviceId = 0;

            for (int i = 0; i < numMessages; i++)
            {
                if (i % MessagesPerClient == 0)
                {
                    deviceId++;
                }

                string exceptionIndex = Random.Next(AllExceptions.Count).ToString();
                byte[] messagebody = PossibleMessageBodies[Random.Next(PossibleMessageBodies.Count)];
                Dictionary<string, string> propertiesContents = new Dictionary<string, string>(PossibleMessagePropertiesContents[Random.Next(PossibleMessagePropertiesContents.Count)]);
                propertiesContents.Add(ExceptionIndexPlaceholder, exceptionIndex);
                propertiesContents.Add(MessageOrderingPlaceholder, (i%MessagesPerClient).ToString());

                messagePool.Add(
                    new Message(TelemetryMessageSource.Instance, messagebody, propertiesContents, new Dictionary<string, string>
                    {
                        [ClientIdentityPlaceholder] = deviceId.ToString()
                    }, 4));
            }

            return messagePool;
        }

        Mock<ICloudProxy> CreateCloudProxyMock(int testId) 
        {
            double probabilityOfException = Random.NextDouble();
            var sequence = new MockSequence();
            var cloudProxy = new Mock<ICloudProxy>();

            Action<List<IEdgeMessage>> throwExceptionRandomly = (messages) => {
                string exceptionIndexToBeThrown = messages[0].Properties[ExceptionIndexPlaceholder];
                string clientIdentity = messages[0].SystemProperties[ClientIdentityPlaceholder];
                string exceptionDescription = AllExceptions[int.Parse(exceptionIndexToBeThrown)].GetType().ToString();
                string firstMessageSequenceNumber = messages[0].Properties[MessageOrderingPlaceholder];
                string lastMessageSequenceNumber = messages[messages.Count-1].Properties[MessageOrderingPlaceholder];
                if (Random.NextDouble() < probabilityOfException)
                // if (0 < probabilityOfException)
                {
                    outputHelper.WriteLine("LOG: Exception thrown {{ client: {0}, firstMessage: {1}, lastMessage: {2}, exception: {3}, testId: {4} }}", clientIdentity, firstMessageSequenceNumber, lastMessageSequenceNumber, exceptionDescription, testId);
                    throw AllExceptions[int.Parse(exceptionIndexToBeThrown)];
                }
                outputHelper.WriteLine("LOG: Successfully sent {{ client: {0}, firstMessage: {1}, lastMessage: {2}, exception: {3}, testId: {4} }}", clientIdentity, firstMessageSequenceNumber, lastMessageSequenceNumber, exceptionDescription, testId);
            };

            cloudProxy.Setup(c => c.SendMessageAsync(It.IsAny<IEdgeMessage>()))
            .Returns(new Func<IEdgeMessage, Task>( message => {
                throwExceptionRandomly(new List<IEdgeMessage> { message });
                return Task.CompletedTask;
            }));

            cloudProxy.Setup(c => c.SendMessageBatchAsync(It.IsAny<IEnumerable<IEdgeMessage>>()))
            .Returns(new Func<IEnumerable<IEdgeMessage>, Task> ( messagesEnumerable => {
                IEnumerator<IEdgeMessage> messagesEnumerator = messagesEnumerable.GetEnumerator();
                messagesEnumerator.MoveNext();
                IEdgeMessage firstMessage = messagesEnumerator.Current;
                IEdgeMessage lastMessage = firstMessage;
                while (messagesEnumerator.MoveNext())
                {
                    lastMessage = messagesEnumerator.Current;
                }
                throwExceptionRandomly.Invoke(new List<IEdgeMessage> {firstMessage, lastMessage});
                return Task.CompletedTask;
            }));

            return cloudProxy;
        }

        bool isMessageOrderValid(List<IMessage> messages)
        {
            if (messages.Count == 0)
            {
                outputHelper.WriteLine("WARNING: No messages recorded in checkpointer");
                return true;
            }

            Dictionary<string, List<int>> clientToMessageSequenceNumbers = new Dictionary<string, List<int>>();
            foreach (IMessage currMessage in messages)
            {
                string currClient = currMessage.SystemProperties[ClientIdentityPlaceholder];
                int currSeqNum = int.Parse(currMessage.Properties[MessageOrderingPlaceholder]);

                if (!clientToMessageSequenceNumbers.ContainsKey(currClient))
                {
                    clientToMessageSequenceNumbers.Add(currClient, new List<int> {currSeqNum});
                    continue;
                }

                List<int> clientSequenceNumbers = clientToMessageSequenceNumbers[currClient];
                int prevSeqNum = clientSequenceNumbers[clientSequenceNumbers.Count - 1];
                if (currSeqNum <= prevSeqNum) { 
                    outputHelper.WriteLine("ERROR: Messages out of order {{ prevSeqNum: {0}, currSeqNum: {1} }}", prevSeqNum, currSeqNum);
                    return false;
                }
                clientSequenceNumbers.Add(currSeqNum);
            }

            return true;
        }

        bool hasSameMessages(List<IMessage> sentMessages, List<IMessage> checkpointedMessages)
        {
            if (sentMessages.Count != checkpointedMessages.Count)
            {
                outputHelper.WriteLine("ERROR: checkpointer has processed more messages than originally sent");
                return false;
            }

            HashSet<IMessage> checkpointedMessagesSet = new HashSet<IMessage>(checkpointedMessages);
            bool isSuccess = true;
            foreach (IMessage msg in sentMessages)
            {
                if (! checkpointedMessagesSet.Contains(msg))
                {
                    string missedMessageSeqNum = msg.Properties[MessageOrderingPlaceholder];
                    outputHelper.WriteLine("ERROR: detected message not processed in checkpointer {{ missedMessageSeqNum: {0} }}", missedMessageSeqNum);
                    isSuccess = false;
                }
            }
            return isSuccess;
        }

        [Theory]
        [MemberData(nameof(GetFsmConfigurations))]
        public void TesEventExecutorFsmFuzz(int numClients, int fanout, int batchSize, int testId)
        {
            outputHelper.WriteLine("{{ numClients: {0}, fanout: {1}, batchSize: {2} }}", numClients, fanout, batchSize);

            List<IMessage> messagePool = CreateMessagePool(numClients);
            LoggedCheckpointer checkpointer = new LoggedCheckpointer(Checkpointer.CreateAsync("checkpointer", new NullCheckpointStore(0L)).Result);

            Mock<ICloudProxy> cloudProxy = CreateCloudProxyMock(testId);
            string cloudEndpointId = "fuzzEndpoint";
            var cloudEndpoint = new CloudEndpoint(cloudEndpointId, _ => Task.FromResult(Option.Some(cloudProxy.Object)), new RoutingMessageConverter(), batchSize, fanout);

            var messageStore = new TestMessageStore();
            var asyncEndpointExecutorOptions = new AsyncEndpointExecutorOptions(10);
            var storingAsyncEndpointExecutor = new StoringAsyncEndpointExecutor(cloudEndpoint, checkpointer, EndpointExecutorConfig, asyncEndpointExecutorOptions, messageStore);

            foreach (IMessage msg in messagePool.ToArray())
            {
                storingAsyncEndpointExecutor.Invoke(msg).Wait();
            }

            // TODO: Move TestMessageStore to new file
            while ( ((TestMessageQueue) messageStore.GetMessageIterator(cloudEndpointId)).index != messagePool.Count)
            {
                Task.Delay(500).Wait();
            }

            Assert.NotEqual(State.DeadIdle, storingAsyncEndpointExecutor.Status.State);
            Assert.True(isMessageOrderValid((List<IMessage>) checkpointer.Processed));
            Assert.True(hasSameMessages(messagePool, (List<IMessage>) checkpointer.Processed));

            // TODO: Assert correct states
            // TODO: assert that other conditions in Executor.Status are appropriate
            // Assert.Equal(4L, checkpointer.Offset);
        }

        public static IEnumerable<object[]> GetFsmConfigurations()
        {
            int testId = 0;
            foreach (int numClients in PossibleNumberOfClients)
            {
                foreach (int fanout in PossibleFanouts)
                {
                    foreach (int batchSize in PossibleBatchSizes)
                    {
                        testId += 1;
                        yield return new object[] {numClients, fanout, batchSize, testId};
                    }
                }
            }
        }
    }
}