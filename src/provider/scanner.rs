use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

pub type RepoMap = HashMap<(String, String, String), ProviderScannedRepoData>;

pub struct ProviderScannerData {
    pub repos: Arc<RwLock<RepoMap>>,
    pub target_branches: Vec<String>,
}

pub struct ProviderScannedRepoData {
    pub version: String,
}
