use std::io::Error;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use mqtt3::proto::Publication;
use parking_lot::Mutex;
use tracing::debug;

use crate::persist::Persist;

/// In memory persistence implementation used for the bridge
struct DiskPersist {}

#[async_trait]
impl Persist for DiskPersist {}

#[cfg(test)]
mod tests {}
