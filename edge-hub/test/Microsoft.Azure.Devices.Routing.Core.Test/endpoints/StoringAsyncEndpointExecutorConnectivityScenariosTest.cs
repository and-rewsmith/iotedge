// Copyright (c) Microsoft. All rights reserved.
namespace Microsoft.Azure.Devices.Routing.Core.Test.Endpoints
{
    using System;
    using System.Collections.Generic;
    using System.Diagnostics.CodeAnalysis;
    using System.Linq;
    using System.Threading.Tasks;
    using Microsoft.Azure.Devices.Client.Exceptions;
    using Microsoft.Azure.Devices.Edge.Hub.Core.Cloud;
    using Microsoft.Azure.Devices.Edge.Hub.Core.Routing;
    using Microsoft.Azure.Devices.Edge.Util;
    using Microsoft.Azure.Devices.Edge.Util.TransientFaultHandling;
    using Microsoft.Azure.Devices.Routing.Core;
    using Microsoft.Azure.Devices.Routing.Core.Checkpointers;
    using Microsoft.Azure.Devices.Routing.Core.Endpoints;
    using Microsoft.Azure.Devices.Routing.Core.Endpoints.StateMachine;
    using Microsoft.Azure.Devices.Routing.Core.MessageSources;
    using Microsoft.Azure.Devices.Routing.Core.Test.Checkpointers;
    using Moq;
    using Xunit;
    using Xunit.Abstractions;
    using IEdgeMessage = Microsoft.Azure.Devices.Edge.Hub.Core.IMessage;

    [ExcludeFromCodeCoverage]
    public class StoringAsyncEndpointExecutorConnectivityScenariosTest : RoutingUnitTestBase
    {
        static readonly string ClientIdentityPlaceholder = "connectionDeviceId";
        static readonly string TransientExceptionIndexPlaceholder = "TransientExceptionIndex";
        static readonly string InvalidExceptionIndexPlaceholder = "InvalidExceptionIndex";
        static readonly string MessageOffsetPlaceholder = "messageOffset";
        static readonly List<Exception> InvalidExceptions = new List<Exception>
                {
                    new ArgumentException("Dummy"),
                    new ArgumentNullException("Dummy"),
                    // new MessageTooLargeException("Dummy"),
                };
        static readonly List<Exception> TransientExceptions = new List<Exception>
                {
                    new IotHubException("Dummy"),
                    // new TimeoutException("Dummy"),
                    // new UnauthorizedException("Dummy"),
                    // new DeviceMaximumQueueDepthExceededException("Dummy"),
                    // new IotHubSuspendedException("Dummy"),
                    // new DeviceAlreadyExistsException("Dummy"),
                    // new DeviceDisabledException("Dummy"),
                    // new DeviceMessageLockLostException("Dummy"),
                    // new IotHubCommunicationException("Dummy"),
                    // new IotHubNotFoundException("Dummy"),
                    // new IotHubThrottledException("Dummy"),
                    // new QuotaExceededException("Dummy"),
                    // new ServerBusyException("Dummy"),
                    // new ServerErrorException("Dummy"),
                };
        private ITestOutputHelper outputHelper;

        public StoringAsyncEndpointExecutorConnectivityScenariosTest(ITestOutputHelper outputHelper)
        {
            this.outputHelper = outputHelper;
        }

        bool IsMessageOrderValid(List<IMessage> messages)
        {
            if (messages.Count == 0)
            {
                this.outputHelper.WriteLine("WARNING: No messages recorded in checkpointer");
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
                bool isCurrInvalid = currMessage.Properties.ContainsKey(InvalidExceptionIndexPlaceholder);
                if (currMessageOffset <= prevMessageOffset && !isCurrInvalid )
                {
                    this.outputHelper.WriteLine("ERROR: Messages out of order {{ prevOffset: {0}, currOffset: {1} }}", prevMessageOffset, currMessageOffset);
                    this.outputHelper.WriteLine("Previous");
                    this.outputHelper.WriteLine(string.Join(";", prevMessage.Properties.Select(x => x.Key + "=" + x.Value).ToArray()));
                    this.outputHelper.WriteLine(string.Join(";", prevMessage.SystemProperties.Select(x => x.Key + "=" + x.Value).ToArray()));
                    this.outputHelper.WriteLine("Current");
                    this.outputHelper.WriteLine(string.Join(";", currMessage.Properties.Select(x => x.Key + "=" + x.Value).ToArray()));
                    this.outputHelper.WriteLine(string.Join(";", currMessage.SystemProperties.Select(x => x.Key + "=" + x.Value).ToArray()));
                    return false;
                }

                clientMessages.Add(currMessage);
            }

            return true;
        }

        bool HasSameMessages(List<IMessage> sentMessages, List<IMessage> checkpointedMessages)
        {
            this.outputHelper.WriteLine("LOG: {{ sentMessages: {0}, checkpointedMessages: {1}}}", sentMessages.Count, checkpointedMessages.Count);

            if (sentMessages.Count != checkpointedMessages.Count)
            {
                this.outputHelper.WriteLine("ERROR: message quantity mismatch");
                return false;
            }

            HashSet<IMessage> checkpointedMessagesSet = new HashSet<IMessage>(checkpointedMessages);
            bool isSuccess = true;
            foreach (IMessage msg in sentMessages)
            {
                if (!checkpointedMessagesSet.Contains(msg))
                {
                    string missedMessageOffset = msg.Properties[MessageOffsetPlaceholder];
                    this.outputHelper.WriteLine("ERROR: detected message not processed in checkpointer {{ missedMessageOffset: {0} }}", MessageOffsetPlaceholder);
                    isSuccess = false;
                }
            }

            return isSuccess;
        }
        Mock<ICloudProxy> CreateCloudProxyMock(MessageGenerationConfig messageGenerationConfig)
        {
            var cloudProxy = new Mock<ICloudProxy>();

            Dictionary<string, int> messageOffsetToCallCounter = new Dictionary<string, int>();

            Action<IEdgeMessage> throwExceptionBasedOnCall = (message) =>
            {
                Exception ex;
                IDictionary<string, string> properties = message.Properties;

                string messageOffset = properties[MessageOffsetPlaceholder];
                if (!messageOffsetToCallCounter.ContainsKey(messageOffset))
                {
                    messageOffsetToCallCounter.Add(messageOffset, 1);
                }
                else
                {
                    messageOffsetToCallCounter[messageOffset] += 1;
                }

                if (properties.ContainsKey(TransientExceptionIndexPlaceholder) && messageOffsetToCallCounter[messageOffset] < messageGenerationConfig.TransientFailureCount)
                {
                    int transientExceptionIndex = int.Parse(properties[TransientExceptionIndexPlaceholder]);
                    ex = TransientExceptions[transientExceptionIndex];
                    throw ex;
                }
                if (properties.ContainsKey(InvalidExceptionIndexPlaceholder))
                {
                    int InvalidExceptionIndex = int.Parse(properties[InvalidExceptionIndexPlaceholder]);
                    ex = InvalidExceptions[InvalidExceptionIndex];
                    throw ex;
                }
            };

            cloudProxy.Setup(c => c.SendMessageAsync(It.IsAny<IEdgeMessage>()))
            .Returns(new Func<IEdgeMessage, Task>(message =>
            {
                throwExceptionBasedOnCall(message);
                return Task.CompletedTask;
            }));

            cloudProxy.Setup(c => c.SendMessageBatchAsync(It.IsAny<IEnumerable<IEdgeMessage>>()))
            .Returns(new Func<IEnumerable<IEdgeMessage>, Task>(messagesEnumerable =>
            {
                // TODO: attempt to clean up this logic
                IEnumerator<IEdgeMessage> messagesEnumerator = messagesEnumerable.GetEnumerator();
                IEdgeMessage firstMessage = messagesEnumerator.Current;
                throwExceptionBasedOnCall(firstMessage);
                return Task.CompletedTask;
            }));

            return cloudProxy;
        }

        [Theory]
        [MemberData(nameof(GetConnectivityScenarios))]
        void TestEndpointExecutorConnectivityScenarios(CloudEndpointConfig cloudEndpointConfig, MessageGenerationConfig messageGenerationConfig)
        {
            // TODO: log configs
            outputHelper.WriteLine(cloudEndpointConfig.ToString());
            outputHelper.WriteLine(messageGenerationConfig.ToString());

            Mock<ICloudProxy> cloudProxy = this.CreateCloudProxyMock(messageGenerationConfig);
            var cloudEndpoint = new CloudEndpoint("testEndpoint", _ => Task.FromResult(Option.Some(cloudProxy.Object)), new RoutingMessageConverter(), cloudEndpointConfig.BatchSize, cloudEndpointConfig.Fanout);

            RetryStrategy maxRetryStrategy = new FixedInterval(int.MaxValue, TimeSpan.FromMilliseconds(1)); // TODO: retry not inf
            EndpointExecutorConfig endpointExecutorConfig = new EndpointExecutorConfig(new TimeSpan(TimeSpan.TicksPerDay), maxRetryStrategy, TimeSpan.FromMinutes(5));

            LoggedCheckpointer checkpointer = new LoggedCheckpointer(Checkpointer.CreateAsync("checkpointer", new NullCheckpointStore(0L)).Result);

            var asyncEndpointExecutorOptions = new AsyncEndpointExecutorOptions(10); 
            var messageStore = new TestMessageStore();

            var storingAsyncEndpointExecutor = new StoringAsyncEndpointExecutor(cloudEndpoint, checkpointer, endpointExecutorConfig, asyncEndpointExecutorOptions, messageStore);

            List<IMessage> messagePool = messageGenerationConfig.GenerateMesssages();
            foreach (IMessage msg in messagePool.ToArray())
            {
                storingAsyncEndpointExecutor.Invoke(msg).Wait();
            }

            List<State> endingStates = new List<State> { State.Idle, State.DeadIdle, State.Closed };
            while (!endingStates.Contains(storingAsyncEndpointExecutor.Status.State))
            {
                Task.Delay(500).Wait();
            }
            Task.Delay(1000).Wait(); // TODO: wait better or make proportional to messages sent

            this.outputHelper.WriteLine("LOG: Executor processing finished. Beginning checks...\n");
            this.outputHelper.WriteLine("LOG: Finite State Machine: {0}", storingAsyncEndpointExecutor.Status.State.ToString());

            Assert.Equal(State.Idle, storingAsyncEndpointExecutor.Status.State);
            Assert.True(this.HasSameMessages(messagePool, (List<IMessage>)checkpointer.Processed));
            Assert.True(this.IsMessageOrderValid((List<IMessage>)checkpointer.Processed));

            this.outputHelper.WriteLine("\n");
        }

        public static IEnumerable<object[]> GetConnectivityScenarios()
        {
            // CloudEndpointConfig cf = new CloudEndpointConfig(2, 2);
            // MessageGenerationConfig mc = new MessageGenerationConfig(1, true, true, true, 1);
            // yield return new object[] { cf, mc };

            List<CloudEndpointConfig> possibleCloudEndpointConfigs = CloudEndpointConfig.GenerateAllConfigurations();
            List<MessageGenerationConfig> possibleMessageConfigs = MessageGenerationConfig.GenerateAllConfigurations();

            foreach (CloudEndpointConfig cloudEndpointConfig in possibleCloudEndpointConfigs)
            {
                foreach (MessageGenerationConfig messageGenerationConfig in possibleMessageConfigs)
                {
                    yield return new object[] { cloudEndpointConfig, messageGenerationConfig };
                }
            }
        }

        class CloudEndpointConfig
        {
            public int BatchSize;
            public int Fanout;
            public CloudEndpointConfig(int batchSize, int fanout)
            {
                this.BatchSize = batchSize;
                this.Fanout = fanout;
            }

            public static List<CloudEndpointConfig> GenerateAllConfigurations()
            {
                List<CloudEndpointConfig> allConfigurations = new List<CloudEndpointConfig>();

                List<int> possibleFanouts = new List<int> { 2, 10 };
                List<int> possibleBatchSizes = new List<int> { 1, 2 };
                foreach (int fanout in possibleFanouts)
                {
                    foreach (int batchSize in possibleBatchSizes)
                    {
                        allConfigurations.Add(new CloudEndpointConfig(batchSize, fanout));
                    }
                }

                return allConfigurations;
            }

            public override string ToString()
            {
                return string.Format("batchSize: {0}, fanout: {1}", this.BatchSize, this.Fanout);
            }
        }

        class MessageGenerationConfig
        {
            public int NumClients;
            public bool HasSuccess;
            public bool HasInvalid;
            public bool HasTransient;
            public int TransientFailureCount;
            static readonly List<int> PossibleNumberOfClients = new List<int> { 1, 2 };
            static readonly List<bool> GenericErrorOptions = new List<bool> {true, false};
            static readonly List<int> PossibleTransientFailureCounts = new List<int> {1, 3}; // TODO: large amount to test retry timeout

            public MessageGenerationConfig(int numClients, bool hasSuccess, bool hasInvalid, bool hasTransient, int transientFailureCount = 0)
            {
                this.NumClients = numClients;
                this.HasSuccess = hasSuccess;
                this.HasInvalid = hasInvalid;
                this.HasTransient = hasTransient;
                this.TransientFailureCount = transientFailureCount;
            }

            public static List<MessageGenerationConfig> GenerateAllConfigurations()
            {
                List<MessageGenerationConfig> allConfigurations = new List<MessageGenerationConfig>();

                foreach (int numClients in PossibleNumberOfClients)
                {
                    foreach (bool hasSuccess in GenericErrorOptions)
                    {
                        foreach (bool hasTransient in GenericErrorOptions)
                        {
                            foreach (bool hasInvalid in GenericErrorOptions)
                            {
                                if (hasTransient)
                                {
                                    foreach(int transientFailureCount in PossibleTransientFailureCounts)
                                    {
                                        allConfigurations.Add(new MessageGenerationConfig(numClients, hasSuccess, hasTransient, hasInvalid, transientFailureCount));
                                    }
                                }
                                else if (hasSuccess || hasInvalid)
                                {
                                    allConfigurations.Add(new MessageGenerationConfig(numClients, hasSuccess, hasTransient, hasInvalid));
                                }
                            }
                        }
                    }
                }

                return allConfigurations;
            }

            List<IMessage> GenerateMessages(int clientId, int startingOffset = 1)
            {
                List<IMessage> messages = new List<IMessage>();

                int messageGroupSize = Math.Max(TransientExceptions.Count, InvalidExceptions.Count);
                int totalMessages = messageGroupSize * 3; // 3 different cases (success, invalid, transient)
                int messageOffset = startingOffset;
                int transientCount = 0;
                int invalidCount = 0;
                for (int count = 0; count < totalMessages; count++)
                {
                    Dictionary<string, string> propertiesContents = new Dictionary<string, string> { { "key1", "value1" } };
                    if (count % 3 == 1 && this.HasInvalid)
                    {
                        propertiesContents.Add(InvalidExceptionIndexPlaceholder, (invalidCount % InvalidExceptions.Count).ToString());
                        invalidCount++;
                    }
                    else if (count % 3 == 2 && this.HasTransient)
                    {
                        propertiesContents.Add(TransientExceptionIndexPlaceholder, (transientCount % TransientExceptions.Count).ToString());
                        transientCount++;
                    }
                    else if (!(count % 3 == 0 && this.HasSuccess))
                    {
                        continue;
                    }
                    propertiesContents.Add(MessageOffsetPlaceholder, messageOffset.ToString()); // We cannot access the message offset directly later as they no longer will be routing messages at runtime, so we need to place it here

                    messages.Add(
                        new Message(
                            TelemetryMessageSource.Instance,
                            new byte[] { 1, 2, 3, 4 },
                            propertiesContents,
                            new Dictionary<string, string>
                            {
                                [ClientIdentityPlaceholder] = clientId.ToString()
                            },
                            messageOffset));
                    messageOffset += 1;
                }

                return messages;
            }

            public List<IMessage> GenerateMesssages()
            {
                List<IMessage> generatedMessages = new List<IMessage>();
                int startingOffset = 1;
                for (int clientId = 0; clientId < this.NumClients; clientId++)
                {
                    List<IMessage> messages = GenerateMessages(clientId, startingOffset);
                    startingOffset += messages.Count;
                    generatedMessages.AddRange(messages);
                }

                return generatedMessages;
            }

            public override string ToString()
            {
                return string.Format("numClients: {0}, hasSuccess: {1}, hasInvalid: {2}, hasTransient: {3}, transientFailureCount: {4}", this.NumClients, this.HasSuccess, this.HasInvalid, this.HasTransient, this.TransientFailureCount);
            }
        }
    }
}
