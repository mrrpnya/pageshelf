use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

#[derive(Clone)]
pub struct OwnerData {}

#[derive(Clone)]
pub struct ProjectData {}

#[derive(Clone)]
pub struct ChannelData {
    pub version: String,
}
use std::hash::Hash;

#[derive(Eq, PartialEq, Hash)]
pub struct ProjectKey {
    pub owner: Box<str>,
    pub project: Box<str>,
}

#[derive(Eq, PartialEq, Hash)]
pub struct ChannelKey {
    pub owner: Box<str>,
    pub project: Box<str>,
    pub channel: Box<str>,
}

pub struct RepoMap {
    // Directed Acyclic Graph (Owner has objects, which have channels) - No islands
    pub owners: HashMap<Box<str>, OwnerData>,
    pub projects: HashMap<ProjectKey, ProjectData>,
    pub channels: HashMap<ChannelKey, ChannelData>,
}

impl RepoMap {
    pub fn new() -> Self {
        Self {
            owners: HashMap::new(),
            projects: HashMap::new(),
            channels: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.owners.clear();
        self.projects.clear();
        self.channels.clear();
    }

    /* ----------------------------------- Get ---------------------------------- */

    pub fn page_count(&self) -> usize {
        self.channels.len()
    }

    pub fn owners(&self) -> impl Iterator<Item = &OwnerData> {
        self.owners.values()
    }

    pub fn get_owner(&self, name: &str) -> Option<&OwnerData> {
        self.owners.get(name)
    }

    pub fn get_project(&self, owner: &str, project: &str) -> Option<&ProjectData> {
        self.projects.get(&ProjectKey {
            owner: owner.into(),
            project: project.into(),
        })
    }

    pub fn get_channel(&self, owner: &str, project: &str, channel: &str) -> Option<&ChannelData> {
        self.channels.get(&ChannelKey {
            owner: owner.into(),
            project: project.into(),
            channel: channel.into(),
        })
    }

    pub fn projects_for_owner(
        &self,
        owner: &str,
    ) -> impl Iterator<Item = (&Box<str>, &ProjectData)> {
        self.projects
            .iter()
            .filter(move |(k, _)| k.owner.as_ref() == owner)
            .map(|f| (&f.0.project, f.1))
    }

    pub fn channels_for_project(
        &self,
        owner: &str,
        project: &str,
    ) -> impl Iterator<Item = (&Box<str>, &ChannelData)> {
        self.channels
            .iter()
            .filter(move |(k, _)| k.owner.as_ref() == owner && k.project.as_ref() == project)
            .map(|f| (&f.0.channel, f.1))
    }

    /* -------------------------------- Contains -------------------------------- */

    pub fn contains_owner(&self, name: &str) -> bool {
        self.owners.contains_key(name)
    }

    pub fn contains_project(&self, owner: &str, project: &str) -> bool {
        self.projects.contains_key(&ProjectKey {
            owner: owner.into(),
            project: project.into(),
        })
    }

    pub fn contains_channel(&self, owner: &str, project: &str, channel: &str) -> bool {
        self.channels.contains_key(&ChannelKey {
            owner: owner.into(),
            project: project.into(),
            channel: channel.into(),
        })
    }

    /* ------------------------- Insert (Create/Update) ------------------------- */

    pub fn insert_owner(&mut self, name: impl Into<Box<str>>, data: OwnerData) {
        self.owners.insert(name.into(), data);
    }

    pub fn insert_project(
        &mut self,
        owner: &str,
        project: impl Into<Box<str>>,
        data: ProjectData,
    ) -> bool {
        if !self.owners.contains_key(owner) {
            return false; // Owner must exist
        }
        let key = ProjectKey {
            owner: owner.into(),
            project: project.into(),
        };
        self.projects.insert(key, data);
        true
    }

    pub fn insert_channel(
        &mut self,
        owner: &str,
        project: &str,
        channel: impl Into<Box<str>>,
        data: ChannelData,
    ) -> bool {
        let project_key = ProjectKey {
            owner: owner.into(),
            project: project.into(),
        };
        if !self.projects.contains_key(&project_key) {
            return false; // Project must exist
        }
        let key = ChannelKey {
            owner: owner.into(),
            project: project.into(),
            channel: channel.into(),
        };
        self.channels.insert(key, data);
        true
    }

    /* --------------------------------- Delete --------------------------------- */

    pub fn delete_owner(&mut self, owner: &str) -> bool {
        if self.owners.remove(owner).is_some() {
            self.projects.retain(|k, _| k.owner.as_ref() != owner);
            self.channels.retain(|k, _| k.owner.as_ref() != owner);
            true
        } else {
            false
        }
    }

    pub fn delete_project(&mut self, owner: &str, project: &str) -> bool {
        let key = ProjectKey {
            owner: owner.into(),
            project: project.into(),
        };
        if self.projects.remove(&key).is_some() {
            self.channels
                .retain(|k, _| !(k.owner.as_ref() == owner && k.project.as_ref() == project));
            true
        } else {
            false
        }
    }

    pub fn delete_channel(&mut self, owner: &str, project: &str, channel: &str) -> bool {
        let key = ChannelKey {
            owner: owner.into(),
            project: project.into(),
            channel: channel.into(),
        };
        self.channels.remove(&key).is_some()
    }
}

pub struct ProviderScannerData {
    pub repos: Arc<RwLock<RepoMap>>,
    pub target_branches: Vec<String>,
}

impl ProviderScannerData {}

pub struct ProviderScannedRepoData {
    pub version: String,
}
