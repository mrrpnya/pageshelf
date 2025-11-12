use std::{
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
    task::Poll,
    time::{Duration, Instant},
};

use futures::{StreamExt, stream::FuturesUnordered};
use hashbrown::{HashMap, HashSet};
use tokio::{
    sync::{Mutex, RwLock, Semaphore},
    task::JoinHandle,
};
use tracing::{Level, debug, error, event, info, warn};

use crate::{
    event::{Event, EventBus},
    ext::Normalizable,
    upstream::{
        Upstream, UpstreamError,
        managers::polled::data::{ChannelData, OwnerData, ProjectData, RepoMap},
        source::{
            AssetListSource, AssetSource, PageListComponentsSource, PageListSource,
            PageVersionSource, VersionedAssetSource,
        },
    },
};

mod data;

// TODO: Improve API and usability

/// Wraps around another upstream and polls it for a new version every now and then, using the latest version it polls.
///
/// This helps with validation, events, etc.
pub struct PolledSource<
    U: VersionedAssetSource + PageVersionSource + AssetListSource + PageListSource + 'static,
> {
    source: Arc<U>,
    analysis: Arc<RwLock<RepoMap>>,
    auto_scan: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

impl<U: VersionedAssetSource + PageVersionSource + PageListSource + AssetListSource + 'static>
    PolledSource<U>
{
    pub fn start(source: Arc<U>, auto_scan_interval: u64, event_bus: EventBus) -> Self {
        let repos = Arc::new(RwLock::new(RepoMap::default()));
        let update_running = Arc::new(AtomicBool::new(false));
        let auto_scan = Arc::new(AtomicBool::new(true));

        Self {
            source: source.clone(),
            auto_scan: auto_scan.clone(),
            analysis: repos.clone(),
            handle: tokio::spawn(Self::auto_scan(
                auto_scan_interval,
                auto_scan,
                update_running.clone(),
                source,
                repos,
                event_bus,
            )),
        }
    }

    async fn auto_scan(
        poll_interval: u64,
        run: Arc<AtomicBool>,
        update_running: Arc<AtomicBool>,
        upstream: Arc<U>,
        repo_storage: Arc<RwLock<RepoMap>>,
        event_bus: EventBus,
    ) {
        let span = tracing::span!(Level::DEBUG, "auto_scan", poll.interval = poll_interval);
        let _span_guard = span.enter();

        let interval_duration = Duration::from_secs(poll_interval);
        let start = tokio::time::Instant::now() + interval_duration;
        let mut interval = tokio::time::interval_at(start, interval_duration);

        loop {
            if !run.load(std::sync::atomic::Ordering::SeqCst) {
                return;
            }

            // Only run a scan if no other scan is in progress
            let already_running = update_running.swap(true, std::sync::atomic::Ordering::SeqCst);
            if !already_running {
                Self::update(upstream.clone(), repo_storage.clone(), event_bus.clone()).await;

                update_running.store(false, std::sync::atomic::Ordering::SeqCst);

                let page_count: usize;
                {
                    let r = repo_storage.read().await;
                    page_count = r.page_count();
                }

                info!(page.count = page_count, "Performed automated Forgejo scan");
            } else {
                debug!("Skipping update because previous update is still running");
            }

            interval.tick().await;
        }
    }

    pub async fn update(upstream: Arc<U>, repo_storage: Arc<RwLock<RepoMap>>, event_bus: EventBus) {
        info!("Starting upstream analysis update...");
        let start_time = Instant::now();

        // Fetch all pages from upstream
        let pages = match upstream.list_pages().await {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to list pages from upstream: {}", e);
                return;
            }
        };
        let (owners, projects, channels) = &*pages;
        info!("Fetched {} pages from upstream", owners.len());

        // Build a set of all upstream channels for deletion check later
        let mut upstream_channels = HashSet::new();
        for ((owner, project), channel) in owners.iter().zip(projects.iter()).zip(channels.iter()) {
            upstream_channels.insert((owner.clone(), project.clone(), channel.clone()));
        }

        // Concurrent update
        let update_count = Arc::new(Mutex::new(0usize));
        let semaphore = Arc::new(Semaphore::new(20));
        let mut tasks: FuturesUnordered<JoinHandle<usize>> = FuturesUnordered::new();

        for ((owner, project), channel) in owners.iter().zip(projects.iter()).zip(channels.iter()) {
            let owner = Arc::<str>::from(owner.clone());
            let project = Arc::<str>::from(project.clone());
            let channel = Arc::<str>::from(channel.clone());

            let upstream = upstream.clone();
            let repo_storage = repo_storage.clone();
            let update_count = update_count.clone();
            let semaphore = semaphore.clone();
            let event_bus = event_bus.clone();

            tasks.push(tokio::spawn(async move {
                let _permit = semaphore
                    .acquire_owned()
                    .await
                    .expect("Failed to acquire semaphore");

                let version = match upstream.get_page_version(&owner, &project, &channel).await {
                    Ok(v) => Arc::<str>::from(v),
                    Err(UpstreamError::NotFound) => return 0usize,
                    Err(e) => {
                        warn!(
                            "Failed to get page version for {}/{}/{}: {}",
                            owner, project, channel, e
                        );
                        return 0usize;
                    }
                };

                info!("Page version: {version}");

                // Check if update needed
                let needs_update = {
                    let repos_read = repo_storage.read().await;
                    match repos_read.get_channel(&owner, &project, &channel) {
                        Some(existing) => existing.version.as_str() != version.as_ref(),
                        None => true,
                    }
                };

                if !needs_update {
                    debug!("Channel {}/{}/{} is up to date", owner, project, channel);
                    return 0usize;
                }

                info!(
                    "Updating channel {}/{}/{} to version {}",
                    owner, project, channel, version
                );

                let mut valid_assets = HashSet::new();
                if let Ok(list) = upstream.list_assets(&owner, &project, &channel).await {
                    for asset in list.iter() {
                        valid_assets.insert(PathBuf::from(asset.to_string()).normalized_relative());
                    }
                } else {
                    error!(
                        "Failed to get list of assets for {}/{}/{}",
                        owner, project, channel
                    );
                }

                // Apply update
                {
                    let mut repos_write = repo_storage.write().await;
                    repos_write.insert_owner(owner.clone(), OwnerData {});
                    repos_write.insert_project(&owner, project.clone(), ProjectData {});
                    repos_write.insert_channel(
                        &owner,
                        &project,
                        channel.clone(),
                        ChannelData {
                            version: version.to_string(),
                            valid_assets,
                        },
                    );
                    event_bus.publish(Event::PageDiscovered {
                        owner,
                        project,
                        channel,
                    });
                }

                // Increment update count
                let mut count = update_count.lock().await;
                *count += 1;
                1usize
            }));
        }

        // Wait for all updates to finish
        while (tasks.next().await).is_some() {}

        // Remove any stale channels
        {
            let mut repo_write = repo_storage.write().await;

            repo_write.retain_channels(|owner, project, channel, _| {
                let exists_upstream = upstream_channels.contains(&(
                    owner.to_string(),
                    project.to_string(),
                    channel.to_string(),
                ));
                if !exists_upstream {
                    event_bus.publish(Event::PageRemoved {
                        owner: owner.clone(),
                        project: project.clone(),
                        channel: channel.clone(),
                    });
                }
                exists_upstream
            });
        }

        let final_update_count = *update_count.lock().await;
        let duration = (Instant::now() - start_time).as_secs_f32();
        info!(
            "Upstream analysis completed: {} channels updated in {:.2} seconds",
            final_update_count, duration
        );
    }
}

impl<U: VersionedAssetSource + PageVersionSource + PageListSource + AssetListSource> Drop
    for PolledSource<U>
{
    fn drop(&mut self) {
        self.auto_scan
            .store(false, std::sync::atomic::Ordering::SeqCst);

        self.handle.abort();
    }
}

/* -------------------------------------------------------------------------- */
/*                                   Source                                   */
/* -------------------------------------------------------------------------- */

impl<U: VersionedAssetSource + PageVersionSource + PageListSource + AssetListSource>
    PageListComponentsSource for PolledSource<U>
{
    async fn list_owners(&self) -> Result<Arc<[String]>, UpstreamError> {
        let repos = self.analysis.read().await;
        Ok(Arc::from(
            repos
                .owners
                .keys()
                .map(|f| f.to_string())
                .collect::<Vec<_>>(),
        ))
    }

    async fn list_projects(&self, owner: &str) -> Result<Arc<[String]>, UpstreamError> {
        let repos = self.analysis.read().await;
        Ok(Arc::from(
            repos
                .projects
                .keys()
                .filter(|f| &*f.owner == owner)
                .map(|f| f.project.to_string())
                .collect::<Vec<_>>(),
        ))
    }

    async fn list_channels(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Arc<[String]>, UpstreamError> {
        let repos = self.analysis.read().await;
        Ok(Arc::from(
            repos
                .channels
                .keys()
                .filter(|f| &*f.owner == owner && &*f.project == project)
                .map(|f| f.channel.to_string())
                .collect::<Vec<_>>(),
        ))
    }
}

impl<U: VersionedAssetSource + PageVersionSource + PageListSource + AssetListSource> PageListSource
    for PolledSource<U>
{
    async fn list_pages(
        &self,
    ) -> Result<Arc<(Arc<[String]>, Arc<[String]>, Arc<[String]>)>, UpstreamError> {
        let owners = self.list_owners().await?;

        let mut all_projects = Vec::new();
        for owner in owners.iter() {
            let projects = self.list_projects(owner).await?;
            all_projects.extend(projects.iter().cloned());
        }
        let all_projects = Arc::from(all_projects.into_boxed_slice());

        let mut all_channels = Vec::new();
        for owner in owners.iter() {
            let projects = self.list_projects(owner).await?;
            for project in projects.iter() {
                let channels = self.list_channels(owner, project).await?;
                all_channels.extend(channels.iter().cloned());
            }
        }
        let all_channels = Arc::from(all_channels.into_boxed_slice());

        Ok(Arc::new((owners, all_projects, all_channels)))
    }
}

impl<U: VersionedAssetSource + PageVersionSource + PageListSource + AssetListSource> AssetListSource
    for PolledSource<U>
{
    async fn list_assets(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> Result<Arc<[String]>, UpstreamError> {
        let r = self.analysis.read().await;
        r.get_channel(owner, project, channel)
            .map_or(Err(UpstreamError::NotFound), |f| {
                Ok(f.valid_assets
                    .iter()
                    .map(|f| f.clone().to_string_lossy().to_string())
                    .collect::<Arc<[_]>>())
            })
    }
}

impl<U: VersionedAssetSource + PageVersionSource + PageListSource + AssetListSource>
    PageVersionSource for PolledSource<U>
{
    async fn get_page_version(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> Result<String, UpstreamError> {
        let repos = self.analysis.read().await;
        match repos.get_channel(owner, project, channel) {
            Some(c) => Ok(c.version.clone()),
            None => Err(UpstreamError::NotFound),
        }
    }
}

impl<U: VersionedAssetSource + PageVersionSource + PageListSource + AssetListSource> AssetSource
    for PolledSource<U>
{
    async fn get_asset_bytes(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        path: &std::path::Path,
    ) -> Result<Arc<[u8]>, crate::upstream::UpstreamError> {
        let path = path.normalized_relative();

        // Ensure the path actually exists
        let version;
        {
            let r = self.analysis.read().await;
            match r.get_channel(owner, project, channel) {
                Some(v) => {
                    if !v.valid_assets.contains(&path) {
                        return Err(UpstreamError::NotFound);
                    }
                    version = v.version.clone();
                }
                None => {
                    debug!("Asset confirmed to not be in repo");
                    return Err(UpstreamError::NotFound);
                }
            }
        }

        self.source
            .get_version_asset_bytes(owner, project, channel, &version, &path)
            .await
    }
}

impl<U: VersionedAssetSource + PageVersionSource + PageListSource + AssetListSource> Upstream
    for PolledSource<U>
{
}

#[cfg(test)]
mod tests {
    use std::{path::Path, sync::Arc};

    use crate::upstream::{
        mock::{MockUpstream, VersionedMockUpstream},
        source::{PageListComponentsSource, PageVersionSource},
    };
    use crate::{event::EventBus, upstream::managers::PolledSource};

    fn make_event_bus() -> EventBus {
        EventBus::default()
    }

    #[tokio::test]
    async fn test_initial_scan_populates_repo() {
        // Initial upstream with an asset
        let base = MockUpstream::default().with_asset(
            "alice",
            "proj1",
            "main",
            Path::new("index.html"),
            b"Hello world",
        );

        let vmu = VersionedMockUpstream::default();
        vmu.update(base).await;

        let upstream = Arc::new(vmu);
        let bus = make_event_bus();

        let polled = PolledSource::start(upstream.clone(), 3600, bus.clone());

        // Perform scan
        PolledSource::update(upstream.clone(), polled.analysis.clone(), bus.clone()).await;

        let ver = polled
            .get_page_version("alice", "proj1", "main")
            .await
            .unwrap();
        assert_eq!(ver, "0"); // VersionedMockUpstream assigns 1.0.0 to first version

        let projs = polled.list_owners().await.unwrap();
        let projs_vec: Vec<String> = projs.iter().cloned().collect();
        assert_eq!(projs_vec, vec!["alice".to_string()]);

        let projs = polled.list_projects("alice").await.unwrap();
        let projs_vec: Vec<String> = projs.iter().cloned().collect();
        assert_eq!(projs_vec, vec!["proj1".to_string()]);

        let chans = polled.list_channels("alice", "proj1").await.unwrap();
        let chans_vec: Vec<String> = chans.iter().cloned().collect();
        assert_eq!(chans_vec, vec!["main".to_string()]);
    }

    #[tokio::test]
    async fn test_new_version_triggers_update() {
        // Initial upstream (v1)
        let v1 =
            MockUpstream::default().with_asset("bob", "proj2", "dev", Path::new("a.txt"), b"aaa");

        // Updated upstream (v2)
        let v2 =
            MockUpstream::default().with_asset("bob", "proj2", "dev", Path::new("a.txt"), b"bbb");

        let vmu = VersionedMockUpstream::default();
        vmu.update(v1).await;

        let upstream = Arc::new(vmu);
        let bus = make_event_bus();

        let polled = PolledSource::start(upstream.clone(), 3600, bus.clone());

        PolledSource::update(upstream.clone(), polled.analysis.clone(), bus.clone()).await;

        let ver1 = polled
            .get_page_version("bob", "proj2", "dev")
            .await
            .unwrap();
        assert_eq!(ver1, "0");

        upstream.update(v2).await;

        PolledSource::update(upstream.clone(), polled.analysis.clone(), bus.clone()).await;

        let ver2 = polled
            .get_page_version("bob", "proj2", "dev")
            .await
            .unwrap();
        assert_eq!(ver2, "1");
    }

    #[tokio::test]
    async fn test_deleted_channel_removes_from_repo() {
        // Initial upstream with two channels
        let v1 = MockUpstream::default()
            .with_asset("charlie", "proj", "stable", Path::new("a.txt"), b"aaa")
            .with_asset("charlie", "proj", "beta", Path::new("b.txt"), b"bbb");
        let v2 = MockUpstream::default().with_asset(
            "charlie",
            "proj",
            "stable",
            Path::new("a.txt"),
            b"aaa",
        );

        let vmu = VersionedMockUpstream::default();
        vmu.update(v1).await;

        let upstream = Arc::new(vmu);
        let bus = make_event_bus();

        let polled = PolledSource::start(upstream.clone(), 3600, bus.clone());

        // Initial scan
        PolledSource::update(upstream.clone(), polled.analysis.clone(), bus.clone()).await;

        let chans_before = polled.list_channels("charlie", "proj").await.unwrap();
        let mut sorted_before: Vec<String> = chans_before.iter().cloned().collect();
        sorted_before.sort();
        assert_eq!(
            sorted_before,
            vec!["beta".to_string(), "stable".to_string()]
        );

        // Update upstream
        let vmu = VersionedMockUpstream::default();
        vmu.update(v2).await;
        let upstream = Arc::new(vmu);

        // Rescan; "beta" should be removed
        PolledSource::update(upstream, polled.analysis.clone(), bus).await;

        let chans_after: Arc<[String]> = polled.list_channels("charlie", "proj").await.unwrap();
        let chans_after: Vec<String> = chans_after.iter().cloned().collect();
        assert_eq!(chans_after, vec!["stable".to_string()]);
    }
}
