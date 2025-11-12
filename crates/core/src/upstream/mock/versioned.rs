use std::{
    collections::{BTreeMap, VecDeque},
    path::Path,
    sync::Arc,
};

use tokio::sync::RwLock;

use crate::upstream::{
    Upstream, UpstreamError,
    source::{
        AssetListSource, AssetSource, PageListComponentsSource, PageListSource, PageVersionSource,
        VersionedAssetSource,
    },
};

use super::MockUpstream;

// TODO: Cleanup

/// A wrapper that manages multiple [`MockUpstream`] snapshots across versions.
///
/// It's primarily meant for testing.
#[derive(Debug, Default)]
pub struct VersionedMockUpstream {
    /// Preloaded to accelerate zero-history access
    empty: Arc<MockUpstream>,
    history: Arc<RwLock<VecDeque<(Arc<MockUpstream>, String)>>>,
    current_index: Arc<RwLock<usize>>,
    /// Tracks what version a given channel is on
    channel_versions: Arc<RwLock<BTreeMap<(String, String, String), usize>>>,
}

impl VersionedMockUpstream {
    /// Add a new snapshot. Automatically increments versions for channels that changed.
    ///
    /// Returns the version of this update.
    pub async fn update(&self, next: MockUpstream) -> String {
        let mut history = self.history.write().await;
        let mut versions = self.channel_versions.write().await;

        let prev = history.back();

        let owners = next.list_owners().await.unwrap();
        for owner in owners.iter() {
            let projects = next.list_projects(owner).await.unwrap();
            for project in projects.iter() {
                let channels = next.list_channels(owner, project).await.unwrap();
                for channel in channels.iter() {
                    let key = (owner.clone(), project.clone(), channel.clone());

                    if prev.is_none() {
                        // First
                        versions.insert(key, 0);
                    } else {
                        // Update
                        let counter = versions.get(&key).copied().unwrap_or(0) + 1;
                        versions.insert(key, counter);
                    }
                }
            }
        }

        // TODO: More human-friendly version strings?
        let version = history.len().to_string();

        history.push_back((Arc::new(next), version.clone()));
        *self.current_index.write().await = history.len() - 1;

        version
    }

    /// Gets the current latest snapshot available
    async fn current(&self) -> Arc<MockUpstream> {
        let idx = *self.current_index.read().await;
        match self.history.read().await.get(idx).cloned() {
            Some(data) => data.0,
            None => self.empty.clone(),
        }
    }

    /// Get the current version of a channel
    async fn current_version(&self, owner: &str, project: &str, channel: &str) -> Option<String> {
        let versions = self.channel_versions.read().await;
        let history = self.history.read().await;

        let key = (owner.to_string(), project.to_string(), channel.to_string());
        if !versions.contains_key(&key) {
            return None;
        }
        versions
            .get(&(owner.to_string(), project.to_string(), channel.to_string()))
            .map(|f| history.get(*f).map(|f| f.1.clone()))
            .unwrap_or(None)
    }

    /// Find a snapshot by version number (returns first snapshot containing that version)
    async fn find_by_version(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        version: &str,
    ) -> Option<Arc<MockUpstream>> {
        let history_guard = self.history.read().await;
        if let Some(upstream) = history_guard
            .iter()
            .find(|&f| f.1 == version)
            .cloned()
            .map(|f| f.0)
        {
            return match upstream.has_page(owner, project, channel).await {
                Ok(true) => Some(upstream),
                _ => None,
            };
        }

        None
    }
}

/* -------------------------------------------------------------------------- */
/*                        Source Trait Implementations                        */
/* -------------------------------------------------------------------------- */

impl Upstream for VersionedMockUpstream {}

impl PageListComponentsSource for VersionedMockUpstream {
    async fn list_owners(&self) -> Result<Arc<[String]>, UpstreamError> {
        self.current().await.list_owners().await
    }

    async fn list_projects(&self, owner: &str) -> Result<Arc<[String]>, UpstreamError> {
        self.current().await.list_projects(owner).await
    }

    async fn list_channels(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Arc<[String]>, UpstreamError> {
        self.current().await.list_channels(owner, project).await
    }
}

impl PageVersionSource for VersionedMockUpstream {
    async fn get_page_version(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> Result<String, UpstreamError> {
        self.current_version(owner, project, channel)
            .await
            .map_or(Err(UpstreamError::NotFound), |f| Ok(f.to_string()))
    }
}

impl AssetSource for VersionedMockUpstream {
    async fn get_asset_bytes(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        path: &Path,
    ) -> Result<Arc<[u8]>, UpstreamError> {
        self.current()
            .await
            .get_asset_bytes(owner, project, channel, path)
            .await
    }
}

impl VersionedAssetSource for VersionedMockUpstream {
    async fn get_version_asset_bytes(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        version: &str,
        path: &Path,
    ) -> Result<Arc<[u8]>, UpstreamError> {
        if let Some(snapshot) = self.find_by_version(owner, project, channel, version).await {
            snapshot
                .get_asset_bytes(owner, project, channel, path)
                .await
        } else {
            Err(UpstreamError::NotFound)
        }
    }
}

impl AssetListSource for VersionedMockUpstream {
    async fn list_assets(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> Result<Arc<[String]>, UpstreamError> {
        self.current()
            .await
            .list_assets(owner, project, channel)
            .await
    }
}

impl PageListSource for VersionedMockUpstream {
    async fn list_pages(
        &self,
    ) -> Result<Arc<(Arc<[String]>, Arc<[String]>, Arc<[String]>)>, UpstreamError> {
        self.current().await.list_pages().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[tokio::test]
    async fn test_update_and_current() {
        let upstream1 = MockUpstream::default().with_asset(
            "alice",
            "proj",
            "stable",
            Path::new("a.txt"),
            b"v1",
        );

        let upstream2 = MockUpstream::default().with_asset(
            "alice",
            "proj",
            "stable",
            Path::new("a.txt"),
            b"v2",
        );

        let vmu = VersionedMockUpstream::default();
        vmu.update(upstream1).await;

        // Current version after initialization should be "0"
        assert_eq!(
            vmu.get_page_version("alice", "proj", "stable")
                .await
                .unwrap(),
            "0"
        );

        // Update to second snapshot
        vmu.update(upstream2).await;

        // Current version should now be "1"
        assert_eq!(
            vmu.get_page_version("alice", "proj", "stable")
                .await
                .unwrap(),
            "1"
        );
    }

    #[tokio::test]
    async fn test_multiple_branches() {
        let upstream1 = MockUpstream::default()
            .with_asset("alice", "proj", "stable", Path::new("a.txt"), b"stable")
            .with_asset("alice", "proj", "beta", Path::new("a.txt"), b"beta");

        let vmu = VersionedMockUpstream::default();
        vmu.update(upstream1).await;

        let channels = vmu.list_channels("alice", "proj").await.unwrap();
        let mut channels: Vec<String> = channels.iter().cloned().collect();
        channels.sort();
        assert_eq!(channels, vec!["beta".to_string(), "stable".to_string()])
    }

    #[tokio::test]
    async fn test_get_version_asset_bytes() {
        let upstream1 = MockUpstream::default().with_asset(
            "alice",
            "proj",
            "stable",
            Path::new("a.txt"),
            b"v0",
        );
        let upstream2 = MockUpstream::default().with_asset(
            "alice",
            "proj",
            "stable",
            Path::new("a.txt"),
            b"v1",
        );

        let vmu = VersionedMockUpstream::default();
        vmu.update(upstream1).await;
        vmu.update(upstream2).await;

        // Request by explicit version
        let bytes = vmu
            .get_version_asset_bytes("alice", "proj", "stable", "0", Path::new("a.txt"))
            .await
            .unwrap();
        assert_eq!(&*bytes, b"v0");

        let bytes = vmu
            .get_version_asset_bytes("alice", "proj", "stable", "1", Path::new("a.txt"))
            .await
            .unwrap();
        assert_eq!(&*bytes, b"v1");

        // Nonexistent version should return NotFound
        assert!(matches!(
            vmu.get_version_asset_bytes("alice", "proj", "stable", "3", Path::new("a.txt"))
                .await,
            Err(UpstreamError::NotFound)
        ));
    }
}
