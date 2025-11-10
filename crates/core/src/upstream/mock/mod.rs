//! Upstream mocking module
//!
//! Provides utilities to test with [Upstream].

use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::Arc,
};

use crate::{
    ext::Normalizable,
    upstream::{Upstream, UpstreamError},
};

/* -------------------------------------------------------------------------- */
/*                         MockUpstream implementation                        */
/* -------------------------------------------------------------------------- */

/// Stores the set of projects an owner has.
#[derive(Default, Debug, PartialEq, Eq)]
struct OwnerData {
    projects: HashSet<String>,
}

/// Stores the set of channels a project has.
#[derive(Default, Debug, PartialEq, Eq)]
struct ProjectData {
    channels: HashSet<String>,
}

#[derive(Default, Debug, PartialEq, Eq)]
struct ChannelData {
    assets: HashSet<String>,
    version: String,
}

/// A fake [Upstream] that stores assets in memory.
///
/// Can be used for testing components that utilize [Upstream] to do things.
#[derive(Default, Debug, PartialEq, Eq)]
pub struct MockUpstream {
    owners: HashMap<String, OwnerData>,
    projects: HashMap<(String, String), ProjectData>,
    channels: HashMap<(String, String, String), ChannelData>,
    assets: HashMap<(String, String, String, String), Arc<[u8]>>, // (owner, project, channel, path)
}

impl MockUpstream {
    /// Registers a given asset.
    ///
    /// This will override existing assets.
    pub fn with_asset(
        mut self,
        owner_name: &str,
        project_name: &str,
        channel_name: &str,
        path: &Path,
        asset: &[u8],
    ) -> Self {
        // Ensure owner exists
        self.owners
            .entry(owner_name.to_string())
            .or_default()
            .projects
            .insert(project_name.to_string());

        // Ensure project exists
        self.projects
            .entry((owner_name.to_string(), project_name.to_string()))
            .or_default()
            .channels
            .insert(channel_name.to_string());

        // Ensure channel exists
        self.channels
            .entry((
                owner_name.to_string(),
                project_name.to_string(),
                channel_name.to_string(),
            ))
            .or_default()
            .assets
            .insert(path.to_string_lossy().to_string());

        // Store the asset
        self.assets.insert(
            (
                owner_name.to_string(),
                project_name.to_string(),
                channel_name.to_string(),
                path.normalized_relative()
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            ),
            Arc::from(asset.to_vec().into_boxed_slice()),
        );

        self
    }

    /// Sets the version of a given page.
    pub fn with_version(
        mut self,
        owner: &str,
        project: &str,
        channel: &str,
        version: &str,
    ) -> Self {
        self.channels
            .entry((owner.to_string(), project.to_string(), channel.to_string()))
            .or_default()
            .version = version.to_string();
        self
    }
}

impl Upstream for MockUpstream {
    #[allow(async_fn_in_trait)]
    async fn get_asset_bytes(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        path: &Path,
    ) -> Result<Arc<[u8]>, UpstreamError> {
        // Processes path stuff (/../, /./), ensures no leading slash
        let path_str = match path.normalized_relative().into_os_string().into_string() {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Error interpreting path: {e:?}");
                return Err(UpstreamError::ProviderError);
            }
        };

        let key = (
            owner.to_owned(),
            project.to_owned(),
            channel.to_owned(),
            path_str,
        );

        // Direct lookup by known keys (faster than iterating)
        match self.assets.get(&key) {
            Some(v) => Ok(v.clone()),
            None => Err(UpstreamError::NotFound),
        }
    }

    async fn get_page_version(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> Result<String, UpstreamError> {
        if let Some(channel_data) =
            self.channels
                .get(&(owner.to_string(), project.to_string(), channel.to_string()))
        {
            Ok(channel_data.version.clone())
        } else {
            Err(UpstreamError::NotFound)
        }
    }

    async fn list_owners(&self) -> Result<Arc<[String]>, UpstreamError> {
        Ok(self.owners.keys().map(|f| f.to_string()).collect())
    }

    async fn list_projects(&self, owner: &str) -> Result<Arc<[String]>, UpstreamError> {
        if let Some(data) = self.owners.get(owner) {
            Ok(data.projects.iter().map(|f| f.to_string()).collect())
        } else {
            Err(UpstreamError::NotFound)
        }
    }

    async fn list_channels(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Arc<[String]>, UpstreamError> {
        if let Some(project_data) = self.projects.get(&(owner.to_string(), project.to_string())) {
            let channels = project_data.channels.iter().cloned().collect(); // Convert HashSet to Vec for iterator
            Ok(channels)
        } else {
            Err(UpstreamError::NotFound)
        }
    }

    async fn list_assets(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> Result<Arc<[String]>, UpstreamError> {
        if let Some(channel_data) =
            self.channels
                .get(&(owner.to_string(), project.to_string(), channel.to_string()))
        {
            Ok(channel_data.assets.iter().cloned().collect())
        } else {
            Err(UpstreamError::NotFound)
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                                    Tests                                   */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use std::path::Path;

    /* -------------------------- Basic Creation Tests -------------------------- */

    #[rstest]
    #[case("alice", "proj1", "main", "index.html", "hello world")]
    #[tokio::test]
    async fn test_with_asset_creates_entries(
        #[case] owner: &str,
        #[case] project: &str,
        #[case] channel: &str,
        #[case] path: &str,
        #[case] content: &str,
    ) {
        let src = MockUpstream::default().with_asset(
            owner,
            project,
            channel,
            Path::new(path),
            content.as_bytes(),
        );

        // owner exists
        let owners: Arc<[_]> = src.list_owners().await.unwrap();
        assert_eq!(*owners, [owner.to_string()]);

        // project exists
        let projects: Arc<[_]> = src.list_projects(owner).await.unwrap();
        assert_eq!(*projects, [project.to_string()]);

        // channel exists
        let channels: Arc<[_]> = src.list_channels(owner, project).await.unwrap();
        assert_eq!(*channels, [channel.to_string()]);

        // asset retrieval works
        let retrieved = src
            .get_asset_bytes(owner, project, channel, Path::new(path))
            .await
            .unwrap();
        assert_eq!(&*retrieved, content.as_bytes());
    }

    #[rstest]
    #[tokio::test]
    async fn test_list_assets() {
        let src = MockUpstream::default()
            .with_asset("amy", "proj", "main", Path::new("a.txt"), b"a")
            .with_asset("amy", "proj", "main", Path::new("b.txt"), b"b")
            .with_asset("amy", "proj", "dev", Path::new("dev.txt"), b"dev");

        let main_assets: Arc<[_]> = src.list_assets("amy", "proj", "main").await.unwrap();
        assert_eq!(main_assets.len(), 2);
        assert!(main_assets.contains(&"a.txt".to_string()));
        assert!(main_assets.contains(&"b.txt".to_string()));

        let dev_assets: Arc<[_]> = src.list_assets("amy", "proj", "dev").await.unwrap();
        assert_eq!(*dev_assets, ["dev.txt".to_string()]);
    }

    /* -------------------------- Duplication Handling -------------------------- */

    #[rstest]
    #[tokio::test]
    async fn test_with_asset_merges_existing() {
        let asset1 = b"first";
        let asset2 = b"second";

        let src = MockUpstream::default()
            .with_asset("bob", "proj2", "beta", Path::new("file1.txt"), asset1)
            .with_asset("bob", "proj2", "beta", Path::new("file2.txt"), asset2);

        // Only one owner
        let owners: Arc<[_]> = src.list_owners().await.unwrap();
        assert_eq!(*owners, ["bob".to_string()]);

        // Only one project
        let projects: Arc<[_]> = src.list_projects("bob").await.unwrap();
        assert_eq!(*projects, ["proj2".to_string()]);

        // Single channel
        let channels: Arc<[_]> = src.list_channels("bob", "proj2").await.unwrap();
        assert_eq!(*channels, ["beta".to_string()]);

        // Both assets retrievable
        let bytes1 = src
            .get_asset_bytes("bob", "proj2", "beta", Path::new("file1.txt"))
            .await
            .unwrap();
        assert_eq!(&*bytes1, asset1.as_slice());

        let bytes2 = src
            .get_asset_bytes("bob", "proj2", "beta", Path::new("file2.txt"))
            .await
            .unwrap();
        assert_eq!(&*bytes2, asset2.as_slice());
    }

    /* ---------------------------- Missing Entities ---------------------------- */

    #[rstest]
    #[tokio::test]
    async fn test_list_projects_not_found() {
        let src = MockUpstream::default();
        let result = src.list_projects("ghost").await;
        assert!(matches!(result, Err(UpstreamError::NotFound)));
    }

    #[rstest]
    #[tokio::test]
    async fn test_list_channels_not_found() {
        let src =
            MockUpstream::default().with_asset("ann", "proj", "main", Path::new("foo.txt"), b"hi");
        let result = src.list_channels("ann", "other").await;
        assert!(matches!(result, Err(UpstreamError::NotFound)));
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_asset_bytes_not_found() {
        let src =
            MockUpstream::default().with_asset("john", "proj", "main", Path::new("foo.txt"), b"hi");
        let err = src
            .get_asset_bytes("john", "proj", "main", Path::new("missing.txt"))
            .await
            .unwrap_err();
        assert!(matches!(err, UpstreamError::NotFound));
    }

    /* ---------------------------- Multiple Channels --------------------------- */

    #[rstest]
    #[tokio::test]
    async fn test_multiple_channels() {
        let src = MockUpstream::default()
            .with_asset("zoe", "proj3", "main", Path::new("a.txt"), b"A")
            .with_asset("zoe", "proj3", "beta", Path::new("b.txt"), b"B");

        let channels: Arc<[_]> = src.list_channels("zoe", "proj3").await.unwrap();
        assert_eq!(channels.len(), 2);
        assert!(channels.contains(&"main".to_string()));
        assert!(channels.contains(&"beta".to_string()));

        let bytes_a = src
            .get_asset_bytes("zoe", "proj3", "main", Path::new("a.txt"))
            .await
            .unwrap();
        assert_eq!(&*bytes_a, b"A");

        let bytes_b = src
            .get_asset_bytes("zoe", "proj3", "beta", Path::new("b.txt"))
            .await
            .unwrap();
        assert_eq!(&*bytes_b, b"B");
    }
}
