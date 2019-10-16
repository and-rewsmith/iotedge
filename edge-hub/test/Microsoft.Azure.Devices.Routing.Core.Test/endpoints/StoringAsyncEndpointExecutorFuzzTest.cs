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
        static readonly List<int> PossibleNumberOfClients = new List<int> {1, 2}; // TODO: add clients
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
                new ArgumentException("Dummy"),
                new ArgumentNullException("Dummy"),
                new DeviceAlreadyExistsException("Dummy"),
                new DeviceDisabledException("Dummy"),
                new DeviceMessageLockLostException("Dummy"),
                new IotHubCommunicationException("Dummy"),
                new IotHubNotFoundException("Dummy"),
                new IotHubThrottledException("Dummy"),
                new MessageTooLargeException("Dummy"),
                new QuotaExceededException("Dummy"),
                new ServerBusyException("Dummy"),
                new ServerErrorException("Dummy"),
            };
        static readonly int MessagesPerClient = 8; // must be at least 8 to exercise all code paths in the FSM surrounding message send
        static readonly string ClientIdentityPlaceholder = "connectionDeviceId";
        static readonly string ExceptionIndexPlaceholder = "exceptionIndex";
        static readonly string MessageOffsetPlaceholder = "messageOffset";
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
                int messageOffset = i+1;
                propertiesContents.Add(MessageOffsetPlaceholder, messageOffset.ToString()); // We cannot access the message offset directly later as they no longer will be routing messages at runtime 
                propertiesContents.Add(ExceptionIndexPlaceholder, exceptionIndex);

                messagePool.Add(
                    new Message(TelemetryMessageSource.Instance, messagebody, propertiesContents, new Dictionary<string, string>
                    {
                        [ClientIdentityPlaceholder] = deviceId.ToString()
                    }, messageOffset));
                
            }

            return messagePool;
        }

        Mock<ICloudProxy> CreateCloudProxyMock(int testId) 
        {
            double probabilityOfException = Random.NextDouble();
            var cloudProxy = new Mock<ICloudProxy>();

            Action<List<IEdgeMessage>> throwExceptionRandomly = (messages) => {
                string exceptionIndexToBeThrown = messages[0].Properties[ExceptionIndexPlaceholder];
                string clientIdentity = messages[0].SystemProperties[ClientIdentityPlaceholder];
                string firstMessageOffset = messages[0].Properties[MessageOffsetPlaceholder];
                string lastMessageOffset = messages[messages.Count-1].Properties[MessageOffsetPlaceholder];
                string exceptionDescription = AllExceptions[int.Parse(exceptionIndexToBeThrown)].GetType().ToString();
                string timestamp = DateTime.UtcNow.ToString("yyyy-MM-dd HH:mm:ss.fff");
                string messageContext = string.Format("{{ client: {0}, firstOffset: {1}, lastOffset: {2} }}", clientIdentity, firstMessageOffset, lastMessageOffset);
                if (Random.NextDouble() < probabilityOfException)
                {
                    outputHelper.WriteLine("LOG: Throwing exception for     {0} for testId {1} at {2} with {3}", messageContext, testId, timestamp, exceptionDescription);
                    throw AllExceptions[int.Parse(exceptionIndexToBeThrown)];
                }
                outputHelper.WriteLine("LOG: Successfully sent          {0} for testId {1} at {2}", messageContext, testId, timestamp);
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

            Dictionary<string, List<IMessage>> clientToMessages = new Dictionary<string, List<IMessage>>();
            foreach (IMessage currMessage in messages)
            {
                string currClient = currMessage.SystemProperties[ClientIdentityPlaceholder];
                if (!clientToMessages.ContainsKey(currClient))
                {
                    clientToMessages.Add(currClient, new List<IMessage> { currMessage });
                    continue;
                }

                List<IMessage> clientMessages = clientToMessages[currClient];
                IMessage prevMessage = clientMessages[clientMessages.Count - 1];

                int currMessageOffset = int.Parse(currMessage.Properties[MessageOffsetPlaceholder]);
                int prevMessageOffset = int.Parse(prevMessage.Properties[MessageOffsetPlaceholder]);
                int exceptionIndex = int.Parse(currMessage.Properties[ExceptionIndexPlaceholder]);
                if ( currMessageOffset <= prevMessageOffset && !(AllExceptions[exceptionIndex] is ArgumentException) ) { 
                    outputHelper.WriteLine("ERROR: Messages out of order {{ prevOffset: {0}, currOffset: {1} }}", prevMessageOffset, currMessageOffset);
                    return false;
                }
                clientMessages.Add(currMessage);
            }

            return true;
        }

        bool hasSameMessages(List<IMessage> sentMessages, List<IMessage> checkpointedMessages)
        {

            outputHelper.WriteLine("LOG: {{ sentMessages: {0}, checkpointedMessages: {1}}}", sentMessages.Count, checkpointedMessages.Count);

            if (sentMessages.Count != checkpointedMessages.Count)
            {
                outputHelper.WriteLine("ERROR: message quantity mismatch");
                return false;
            }

            HashSet<IMessage> checkpointedMessagesSet = new HashSet<IMessage>(checkpointedMessages);
            bool isSuccess = true;
            foreach (IMessage msg in sentMessages)
            {
                if (! checkpointedMessagesSet.Contains(msg))
                {
                    string missedMessageOffset = msg.Properties[MessageOffsetPlaceholder];
                    outputHelper.WriteLine("ERROR: detected message not processed in checkpointer {{ missedMessageOffset: {0} }}", MessageOffsetPlaceholder);
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

            List<State> endingStates = new List<State> {State.Idle, State.DeadIdle, State.DeadProcess, State.Closed};
            while (!endingStates.Contains(storingAsyncEndpointExecutor.Status.State))
            {
                Task.Delay(500).Wait();
            }

            outputHelper.WriteLine("LOG: Executor processing finished. Beginning checks...\n");
            outputHelper.WriteLine("LOG: Finite State Machine: {0}", storingAsyncEndpointExecutor.Status.State.ToString());

            Assert.Equal(State.Idle, storingAsyncEndpointExecutor.Status.State);
            Assert.True(hasSameMessages(messagePool, (List<IMessage>) checkpointer.Processed));
            Assert.True(isMessageOrderValid((List<IMessage>) checkpointer.Processed));

            outputHelper.WriteLine("\n");
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
                        yield return new object[] {numClients, fanout, batchSize, testId};
                    }
                }
            }
        }
    }
}