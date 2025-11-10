//! Forgejo scanning module
//!
//! Utilities to periodically scan information from a Forgejo instance.
//! This information can then be used to do the following without further API calls:
//! - Validate assets
//! - Detect new versions

use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};

use futures::{StreamExt, stream::FuturesUnordered};
use hashbrown::HashSet;

use crate::scanner::data::{ChannelData, OwnerData, ProjectData, RepoMap};
use forgejo_api::{
    Forgejo,
    structs::{GetTreeQuery, RepoSearchQuery},
};
use pageshelf_core::{
    event::{self, EventBus},
    ext::Normalizable,
};
use tokio::{
    sync::{RwLock, Semaphore},
    task::JoinHandle,
};
use tracing::{Level, debug, error, info, span, warn};

pub mod data;

pub struct ProviderScannerData {
    pub repos: Arc<RwLock<RepoMap>>,
    pub target_branches: Vec<String>,
}

impl ProviderScannerData {}

pub struct ForgejoScanner {
    pub data: ProviderScannerData,
    auto_scan: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

impl Drop for ForgejoScanner {
    fn drop(&mut self) {
        self.auto_scan
            .store(false, std::sync::atomic::Ordering::SeqCst);

        self.handle.abort();
    }
}

impl ForgejoScanner {
    pub fn start(
        forgejo: Arc<Forgejo>,
        target_branches: Vec<String>,
        poll_interval: u64,
        event_bus: EventBus,
    ) -> Self {
        let repos = Arc::new(RwLock::new(RepoMap::default()));
        let update_running = Arc::new(AtomicBool::new(false));
        let auto_scan = Arc::new(AtomicBool::new(true));
        Self {
            data: ProviderScannerData {
                repos: repos.clone(),
                target_branches: target_branches.clone(),
            },
            auto_scan: auto_scan.clone(),
            handle: tokio::spawn(Self::auto_scan(
                poll_interval,
                auto_scan,
                update_running.clone(),
                forgejo,
                repos,
                target_branches,
                event_bus,
            )),
        }
    }

    async fn auto_scan(
        poll_interval: u64,
        run: Arc<AtomicBool>,
        update_running: Arc<AtomicBool>,
        forgejo: Arc<Forgejo>,
        repo_storage: Arc<RwLock<RepoMap>>,
        target_branches: Vec<String>,
        event_bus: EventBus,
    ) {
        let span = span!(Level::DEBUG, "auto_scan", poll.interval = poll_interval);
        let _span_guard = span.enter();

        let interval_duration = Duration::from_secs(poll_interval);
        let start = tokio::time::Instant::now() + interval_duration;
        let mut interval = tokio::time::interval_at(start, interval_duration);

        let target_branches: Arc<Vec<String>> = Arc::from(target_branches);

        loop {
            if !run.load(std::sync::atomic::Ordering::SeqCst) {
                return;
            }

            // Only run update if no other update is in progress
            let already_running = update_running.swap(true, std::sync::atomic::Ordering::SeqCst);
            if !already_running {
                Self::update(
                    forgejo.clone(),
                    repo_storage.clone(),
                    target_branches.clone(),
                    event_bus.clone(),
                )
                .await;

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

    async fn update(
        forgejo: Arc<Forgejo>,
        repo_storage: Arc<RwLock<RepoMap>>,
        target_branches: Arc<Vec<String>>,
        event_bus: EventBus,
    ) {
        use std::collections::HashMap;

        info!("Starting Forgejo analysis update...");
        let start = Instant::now();

        // Fetch all repos from Forgejo
        let upstream_repos = match forgejo
            .repo_search(RepoSearchQuery {
                q: None,
                topic: None,
                include_desc: None,
                uid: None,
                priority_owner_id: None,
                team_id: None,
                starred_by: None,
                private: Some(false),
                is_private: None,
                template: None,
                archived: None,
                mode: None,
                exclusive: None,
                sort: None,
                order: None,
                page: None,
                limit: Some(999999),
            })
            .await
        {
            Ok(repos) => {
                let count = repos.data.as_ref().map_or(0, |v| v.len());
                info!("Fetched {} repositories from Forgejo", count);
                repos.data.unwrap_or_default()
            }
            Err(e) => {
                error!("Failed to fetch Forgejo repos: {}", e);
                return;
            }
        };

        let update_count = Arc::new(tokio::sync::Mutex::new(0usize));
        let semaphore = Arc::new(Semaphore::new(20));
        let mut tasks = FuturesUnordered::new();

        for repo in upstream_repos {
            let forgejo = forgejo.clone();
            let target_branches = target_branches.clone();
            let repo_storage = repo_storage.clone();
            let update_count = update_count.clone();
            let semaphore = semaphore.clone();
            let event_bus = event_bus.clone();

            tasks.push(tokio::spawn(async move {
                let _permit = semaphore
                    .acquire_owned()
                    .await
                    .expect("Failed to acquire semaphore");

                let owner_arc: Arc<str> = match repo.owner.as_ref().and_then(|o| o.login.clone()) {
                    Some(login) => Arc::from(login),
                    None => {
                        warn!("Skipping repo with missing owner: {:?}", repo.name);
                        return (HashSet::new(), HashSet::new(), HashSet::new(), 0);
                    }
                };

                let repo_arc: Arc<str> = match repo.name.clone() {
                    Some(name) => Arc::from(name),
                    None => {
                        warn!("Skipping repo with missing name for owner: {}", owner_arc);
                        return (HashSet::new(), HashSet::new(), HashSet::new(), 0);
                    }
                };

                debug!("Processing repository: {}/{}", owner_arc, repo_arc);

                let mut local_seen_owners = HashSet::new();
                let mut local_seen_projects = HashSet::new();
                let mut local_seen_channels = HashSet::new();
                let mut local_updates = 0;

                // Prepare batch updates
                let mut batch_channels: HashMap<Arc<str>, ChannelData> = HashMap::new();

                local_seen_owners.insert(owner_arc.clone());
                local_seen_projects.insert((owner_arc.clone(), repo_arc.clone()));

                for branch_name in target_branches.iter() {
                    let branch_arc = Arc::from(branch_name.clone());

                    let branch = match forgejo
                        .repo_get_branch(&owner_arc, &repo_arc, branch_name)
                        .await
                    {
                        Ok(b) => b,
                        Err(e) => {
                            debug!(
                                "Failed to fetch branch '{}' for {}/{}: {}",
                                branch_name, owner_arc, repo_arc, e
                            );
                            continue;
                        }
                    };

                    let commit_id: Arc<str> = match branch.commit.and_then(|c| c.id) {
                        Some(id) => Arc::from(id),
                        None => {
                            warn!(
                                "Branch '{}' of {}/{} has no commit ID",
                                branch_name, owner_arc, repo_arc
                            );
                            continue;
                        }
                    };

                    // Check if update needed
                    let needs_update = {
                        let repos_read = repo_storage.read().await;
                        match repos_read.get_channel(&owner_arc, &repo_arc, &branch_arc) {
                            Some(existing) => existing.version.as_str() != commit_id.as_ref(),
                            None => true,
                        }
                    };

                    if !needs_update {
                        debug!(
                            "Branch '{}' of {}/{} is up to date",
                            branch_name, owner_arc, repo_arc
                        );
                        local_seen_channels.insert((
                            owner_arc.clone(),
                            repo_arc.clone(),
                            branch_arc,
                        ));
                        continue;
                    }

                    info!(
                        "Branch '{}' of {}/{} is to be updated",
                        branch_name, owner_arc, repo_arc
                    );

                    // Fetch tree
                    let tree = match forgejo
                        .get_tree(
                            &owner_arc,
                            &repo_arc,
                            &commit_id,
                            GetTreeQuery {
                                recursive: Some(true),
                                page: None,
                                per_page: None,
                            },
                        )
                        .await
                    {
                        Ok(t) => t,
                        Err(e) => {
                            warn!(
                                "Failed to fetch tree for branch '{}' of {}/{}: {}",
                                branch_name, owner_arc, repo_arc, e
                            );
                            continue;
                        }
                    };

                    let mut valid_assets = HashSet::new();
                    if let Some(entries) = tree.tree {
                        for entry in entries {
                            if let Some(path) = entry.path {
                                let buf = PathBuf::from(path).normalized_relative();

                                valid_assets.insert(buf);
                            }
                        }
                    }

                    batch_channels.insert(
                        branch_arc.clone(),
                        ChannelData {
                            version: commit_id.to_string(),
                            valid_assets,
                        },
                    );

                    local_seen_channels.insert((owner_arc.clone(), repo_arc.clone(), branch_arc));
                    local_updates += 1;
                }

                // Apply batch updates
                if !batch_channels.is_empty() {
                    for branch_arc in batch_channels.keys() {
                        let event = pageshelf_core::event::Event::PageAvailable {
                            owner: owner_arc.clone(),
                            project: repo_arc.clone(),
                            channel: branch_arc.clone(),
                        };
                        event_bus.publish(event);
                    }
                    let mut repos_write = repo_storage.write().await;
                    repos_write.insert_owner(owner_arc.clone(), OwnerData {});
                    repos_write.insert_project(&owner_arc, repo_arc.clone(), ProjectData {});
                    for (branch, data) in batch_channels {
                        repos_write.insert_channel(&owner_arc, &repo_arc, branch, data);
                    }
                }

                {
                    let mut count = update_count.lock().await;
                    *count += local_updates;
                }

                (
                    local_seen_owners,
                    local_seen_projects,
                    local_seen_channels,
                    local_updates,
                )
            }));
        }

        // Collect results
        let mut global_seen_owners = HashSet::new();
        let mut global_seen_projects = HashSet::new();
        let mut global_seen_channels = HashSet::new();

        while let Some(result) = tasks.next().await {
            if let Ok((owners, projects, channels, _)) = result {
                global_seen_owners.extend(owners);
                global_seen_projects.extend(projects);
                global_seen_channels.extend(channels);
            }
        }

        // Cleanup stale entries
        {
            let mut repos_write = repo_storage.write().await;

            repos_write.retain_channels(|o, p, c, _| {
                if !global_seen_channels.contains(&(o.clone(), p.clone(), c.clone())) {
                    let event = pageshelf_core::event::Event::PageDeleted {
                        owner: o.clone(),
                        project: p.clone(),
                        channel: c.clone(),
                    };
                    event_bus.publish(event);
                    false
                } else {
                    true
                }
            });

            repos_write
                .retain_projects(|o, p, _| global_seen_projects.contains(&(o.clone(), p.clone())));
            repos_write.retain_owners(|o, _| global_seen_owners.contains(o));
        }

        let duration = (Instant::now() - start).as_secs_f32();
        let final_update_count = *update_count.lock().await;
        info!(
            "Forgejo analysis completed: {} branches updated across {} repositories in {:.2} seconds",
            final_update_count,
            global_seen_projects.len(),
            duration
        );
    }
}
