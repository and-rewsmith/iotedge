// Copyright (c) Microsoft. All rights reserved.

namespace Microsoft.Azure.Devices.Edge.Agent.Service
{
    using System;
    using System.IO;
    using Autofac;
    using Microsoft.Azure.Devices.Edge.Agent.Core;
    using Microsoft.Azure.Devices.Edge.Util;
    using Microsoft.Azure.Devices.Edge.Util.Metrics;
    using Microsoft.Azure.Devices.Edge.Util.Metrics.NullMetrics;
    using Microsoft.Azure.Devices.Edge.Util.Metrics.Prometheus.Net;

    public sealed class MetricsModule : Module
    {
        MetricsConfig metricsConfig;
        string iothubHostname;
        string deviceId;
        string edgeAgentStorageFolder;

        public MetricsModule(MetricsConfig metricsConfig, string iothubHostname, string deviceId, string edgeAgentStorageFolder)
        {
            Preconditions.CheckNotNull(edgeAgentStorageFolder, nameof(edgeAgentStorageFolder));
            this.edgeAgentStorageFolder = edgeAgentStorageFolder;

            if (!Directory.Exists(edgeAgentStorageFolder))
            {
                this.metricsConfig = new MetricsConfig(false, metricsConfig.ListenerConfig);
            }
            else
            {
                this.metricsConfig = Preconditions.CheckNotNull(metricsConfig, nameof(metricsConfig));
            }

            this.iothubHostname = Preconditions.CheckNonWhiteSpace(iothubHostname, nameof(iothubHostname));
            this.deviceId = Preconditions.CheckNonWhiteSpace(deviceId, nameof(deviceId));
        }

        protected override void Load(ContainerBuilder builder)
        {
            builder.Register(c => this.metricsConfig.Enabled ?
                                new MetricsProvider(Constants.EdgeAgentModuleName, this.iothubHostname, this.deviceId, this.edgeAgentStorageFolder) :
                                new NullMetricsProvider() as IMetricsProvider)
                .As<IMetricsProvider>()
                .SingleInstance();

            builder.Register(c => new Util.Metrics.Prometheus.Net.MetricsListener(this.metricsConfig.ListenerConfig, c.Resolve<IMetricsProvider>()))
                .As<IMetricsListener>()
                .SingleInstance();

            base.Load(builder);
        }
    }
}
