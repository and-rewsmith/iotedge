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
                    configuration.GetValue<string>("serviceClientConnectionString", string.Empty),
                    configuration.GetValue<ulong>("messageSizeInBytes", 1024),
                    configuration.GetValue<TransportType>("transportType", TransportType.Mqtt_Tcp_Only),
                    configuration.GetValue<string>("outputName", "output1"));
            });

        Settings(
            string serviceClientConnectionString,
            ulong messageSizeInBytes,
            TransportType transportType,
            string outputName)
        {
            this.ServiceClientConnectionString = serviceClientConnectionString;
            this.MessageSizeInBytes = messageSizeInBytes;
            this.TransportType = transportType;
            this.OutputName = outputName;
        }

        public static Settings Current => DefaultSettings.Value;
        public ulong MessageSizeInBytes { get; }

        [JsonConverter(typeof(StringEnumConverter))]
        public TransportType TransportType { get; }

        public string OutputName { get; }

        public string ServiceClientConnectionString { get; }

        public override string ToString()
        {
            return JsonConvert.SerializeObject(this, Formatting.Indented);
        }
    }
}
