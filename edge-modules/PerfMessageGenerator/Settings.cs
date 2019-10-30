// Copyright (c) Microsoft. All rights reserved.
namespace PerfMessageGenerator
{
    using System;
    using System.IO;
    using Microsoft.Azure.Devices.Client;
    using Microsoft.Extensions.Configuration;
    using Newtonsoft.Json;
    using Newtonsoft.Json.Converters;
    using Newtonsoft.Json.Serialization;

    [JsonObject(NamingStrategyType = typeof(CamelCaseNamingStrategy))]
    public class Settings
    {
        static readonly Lazy<Settings> DefaultSettings = new Lazy<Settings>(
            () =>
            {
                IConfiguration configuration = new ConfigurationBuilder()
                    .SetBasePath(Directory.GetCurrentDirectory())
                    .AddJsonFile("config/settings.json", optional: true)
                    .AddEnvironmentVariables()
                    .Build();

                return new Settings(
                    configuration.GetValue("messageFrequency", TimeSpan.FromMilliseconds(20)),
                    configuration.GetValue<ulong>("messageSizeInBytes", 1024),
                    configuration.GetValue<TransportType>("transportType", TransportType.Amqp_Tcp_Only),
                    configuration.GetValue<string>("outputName", "output1"));
            });

        Settings(
            TimeSpan messageFrequency,
            ulong messageSizeInBytes,
            TransportType transportType,
            string outputName)
        {
            this.MessageFrequency = messageFrequency;
            this.MessageSizeInBytes = messageSizeInBytes;
            this.TransportType = transportType;
            this.OutputName = outputName;
        }

        public static Settings Current => DefaultSettings.Value;

        public TimeSpan MessageFrequency { get; }

        public ulong MessageSizeInBytes { get; }

        [JsonConverter(typeof(StringEnumConverter))]
        public TransportType TransportType { get; }

        public string OutputName { get; }

        public override string ToString()
        {
            return JsonConvert.SerializeObject(this, Formatting.Indented);
        }
    }
}
