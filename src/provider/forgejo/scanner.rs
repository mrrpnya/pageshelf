use std::{
    collections::HashMap,
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};

use forgejo_api::{Forgejo, structs::RepoSearchQuery};
use log::info;
use tokio::{sync::RwLock, task::JoinHandle};

/// Analysis on the current state of a Forgejo instance
pub struct ForgejoScanner {
    pub repos: Arc<RwLock<HashMap<(String, String, String), ForgejoScannedRepo>>>,
    pub target_branches: Vec<String>,
    auto_scan: Arc<AtomicBool>, // TODO: Domain name resolution data
    handle: JoinHandle<()>,
}

pub struct ForgejoScannedRepo {
    pub version: String,
}

impl Drop for ForgejoScanner {
    fn drop(&mut self) {
        self.auto_scan
            .store(false, std::sync::atomic::Ordering::SeqCst);

        self.handle.abort();
    }
}

impl ForgejoScanner {
    pub fn start(forgejo: Arc<Forgejo>, target_branches: Vec<String>, poll_interval: u64) -> Self {
        let repos = Arc::new(RwLock::new(HashMap::new()));
        let auto_scan = Arc::new(AtomicBool::new(true));
        let s = Self {
            repos: repos.clone(),
            target_branches: target_branches.clone(),
            auto_scan: auto_scan.clone(),
            handle: tokio::spawn(Self::auto_scan(
                poll_interval,
                auto_scan,
                forgejo,
                repos,
                target_branches,
            )),
        };

        s
    }

    async fn auto_scan(
        poll_interval: u64,
        run: Arc<AtomicBool>,
        forgejo: Arc<Forgejo>,
        repo_storage: Arc<RwLock<HashMap<(String, String, String), ForgejoScannedRepo>>>,
        target_branches: Vec<String>,
    ) {
        let interval_duration = Duration::from_secs(poll_interval);
        let start = tokio::time::Instant::now() + interval_duration;
        let mut interval = tokio::time::interval_at(start, interval_duration);

        loop {
            if !run.load(std::sync::atomic::Ordering::SeqCst) {
                // Loop break
                return;
            }

            println!(
                "Forgejo auto scan initiated at: {:?}",
                tokio::time::Instant::now()
            );

            Self::update(&forgejo, repo_storage.clone(), &target_branches).await;

            interval.tick().await;
        }
    }

    async fn update(
        forgejo: &Forgejo,
        repo_storage: Arc<RwLock<HashMap<(String, String, String), ForgejoScannedRepo>>>,
        target_branches: &Vec<String>,
    ) {
        info!("Updating Forgejo analysis...");
        let start = Instant::now();

        let upstream_repos = forgejo
            .repo_search(RepoSearchQuery {
                q: None,
                topic: None,
                include_desc: None,
                uid: None,
                priority_owner_id: None,
                team_id: None,
                starred_by: None,
                private: None,
                is_private: None,
                template: None,
                archived: None,
                mode: None,
                exclusive: None,
                sort: None,
                order: None,
                page: None,
                limit: Some(99999),
            })
            .await;

        if upstream_repos.is_err() {
            log::error!(
                "Failed to update Forgejo analysis: {}",
                upstream_repos.unwrap_err()
            );
            return;
        }

        let upstream_repos = upstream_repos.unwrap();

        if upstream_repos.data.is_none() {
            return;
        }

        let mut update_count = 0;

        let mut repos = repo_storage.write().await;
        repos.clear();

        for repo in upstream_repos.data.unwrap() {
            let login = repo.owner.unwrap().login.unwrap();
            let repo_name = repo.name.unwrap();
            for branch_name in target_branches {
                let branch = forgejo
                    .repo_get_branch(&login, &repo_name, branch_name)
                    .await;

                if branch.is_err() {
                    continue;
                }

                let branch = branch.unwrap();

                if branch.commit.is_none() {
                    continue;
                }

                let commit = branch.commit.unwrap();

                if commit.id.is_none() {
                    continue;
                }

                let version = commit.id.unwrap();
                repos.insert(
                    (
                        login.to_string(),
                        repo_name.to_string(),
                        branch_name.to_string(),
                    ),
                    ForgejoScannedRepo {
                        version: version.clone(),
                    },
                );

                update_count += 1;

                log::debug!(
                    "Analyzed {}/{}:{} (version {})",
                    login,
                    repo_name,
                    branch_name,
                    version
                )
            }
        }

        let end = Instant::now();
        let duration = (end - start).as_secs_f32();
        info!(
            "Updated Forgejo analysis (updated {} branches, took {} seconds)",
            update_count, duration
        )
    }
}
