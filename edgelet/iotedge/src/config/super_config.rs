// Copyright (c) Microsoft. All rights reserved.

use std::collections::BTreeMap;

use url::Url;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(super) struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) parent_hostname: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) trust_bundle_cert: Option<Url>,

    #[serde(flatten)]
    pub(super) aziot: aziotctl_common::config::super_config::Config,

    pub(super) agent: edgelet_core::ModuleSpec<edgelet_docker::DockerConfig>,

    pub(super) connect: edgelet_core::Connect,
    pub(super) listen: edgelet_core::Listen,

    #[serde(default)]
    pub(super) watchdog: edgelet_core::WatchdogSettings,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) edge_ca: Option<EdgeCa>,

    pub(super) moby_runtime: MobyRuntime,
}

#[derive(Debug, serde_derive::Deserialize, serde_derive::Serialize)]
pub(super) struct EdgeCa {
    pub(super) cert: Url,
    pub(super) pk: Url,
}

#[derive(Debug, serde_derive::Deserialize, serde_derive::Serialize)]
pub(super) struct MobyRuntime {
    pub(super) uri: Url,
    pub(super) network: edgelet_core::MobyNetwork,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) content_trust: Option<ContentTrust>,
}

#[derive(Debug, serde_derive::Deserialize, serde_derive::Serialize)]
pub struct ContentTrust {
    pub ca_certs: Option<BTreeMap<String, Url>>,
}
