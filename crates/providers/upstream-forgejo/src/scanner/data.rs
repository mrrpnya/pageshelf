//! Forgejo scanned data module
//!
//! Utilities for containing information scanned from a Forgejo instance.

use std::{hash::Hash, path::PathBuf, sync::Arc};

use core::hash::Hasher;
use hashbrown::{HashMap, HashSet};
use std::hash::BuildHasher;

#[derive(Clone)]
pub struct OwnerData {}

#[derive(Clone)]
pub struct ProjectData {}

#[derive(Clone)]
pub struct ChannelData {
    pub version: String,
    pub valid_assets: HashSet<PathBuf>,
}

/* ------------------------------- Project Key ------------------------------ */

#[derive(Eq, PartialEq, Hash, Clone)]
pub struct ProjectKey {
    pub owner: Arc<str>,
    pub project: Arc<str>,
}

/* ------------------------------- Channel Key ------------------------------ */

#[derive(Eq, PartialEq, Hash, Clone)]
pub struct ChannelKey {
    pub owner: Arc<str>,
    pub project: Arc<str>,
    pub channel: Arc<str>,
}

#[derive(Default, Clone)]
pub struct RepoMap {
    pub owners: HashMap<Arc<str>, OwnerData>,
    pub projects: HashMap<ProjectKey, ProjectData>,
    pub channels: HashMap<ChannelKey, ChannelData>,
}

impl RepoMap {
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

    pub fn get_owner(&self, name: &Arc<str>) -> Option<&OwnerData> {
        self.owners.get(name)
    }

    pub fn get_project(&self, owner: &str, project: &str) -> Option<&ProjectData> {
        let hash = {
            let mut s = self.channels.hasher().build_hasher();
            owner.hash(&mut s);
            project.hash(&mut s);
            s.finish()
        };

        self.projects
            .raw_entry()
            .from_hash(hash, |k| {
                k.owner.as_ref() == owner && k.project.as_ref() == project
            })
            .map(|(_, v)| v)
    }

    pub fn get_channel(&self, owner: &str, project: &str, channel: &str) -> Option<&ChannelData> {
        let hash = {
            let mut s = self.channels.hasher().build_hasher();
            owner.hash(&mut s);
            project.hash(&mut s);
            channel.hash(&mut s);
            s.finish()
        };

        self.channels
            .raw_entry()
            .from_hash(hash, |k| {
                k.owner.as_ref() == owner
                    && k.project.as_ref() == project
                    && k.channel.as_ref() == channel
            })
            .map(|(_, v)| v)
    }

    pub fn get_channel_mut(
        &mut self,
        owner: &Arc<str>,
        project: &Arc<str>,
        channel: &Arc<str>,
    ) -> Option<&mut ChannelData> {
        let hash = {
            let mut s = self.channels.hasher().build_hasher();
            owner.hash(&mut s);
            project.hash(&mut s);
            channel.hash(&mut s);
            s.finish()
        };

        let entry = self.channels.raw_entry_mut().from_hash(hash, |k| {
            k.owner == *owner && k.project == *project && k.channel == *channel
        });

        if let hashbrown::hash_map::RawEntryMut::Occupied(entry) = entry {
            Some(entry.into_mut()) // into_mut() returns &mut V with correct lifetime
        } else {
            None
        }
    }

    pub fn projects_for_owner(
        &self,
        owner: &Arc<str>,
    ) -> impl Iterator<Item = (&Arc<str>, &ProjectData)> {
        self.projects
            .iter()
            .filter(move |(k, _)| Arc::ptr_eq(&k.owner, owner))
            .map(|f| (&f.0.project, f.1))
    }

    /// Retain only channels for which the predicate returns true
    pub fn retain_channels<F>(&mut self, mut f: F)
    where
        F: FnMut(&Arc<str>, &Arc<str>, &Arc<str>, &ChannelData) -> bool,
    {
        self.channels
            .retain(|key, data| f(&key.owner, &key.project, &key.channel, data));
    }

    /// Retain only projects for which the predicate returns true
    pub fn retain_projects<F>(&mut self, mut f: F)
    where
        F: FnMut(&Arc<str>, &Arc<str>, &ProjectData) -> bool,
    {
        self.projects
            .retain(|key, data| f(&key.owner, &key.project, data));
    }

    /// Retain only owners for which the predicate returns true
    pub fn retain_owners<F>(&mut self, mut f: F)
    where
        F: FnMut(&Arc<str>, &OwnerData) -> bool,
    {
        self.owners.retain(|key, data| f(key, data));
    }

    pub fn channels_for_project(
        &self,
        owner: &Arc<str>,
        project: &Arc<str>,
    ) -> impl Iterator<Item = (&Arc<str>, &ChannelData)> {
        self.channels
            .iter()
            .filter(move |(k, _)| Arc::ptr_eq(&k.owner, owner) && Arc::ptr_eq(&k.project, project))
            .map(|f| (&f.0.channel, f.1))
    }

    /* -------------------------------- Contains -------------------------------- */

    pub fn contains_owner(&self, name: &Arc<str>) -> bool {
        self.owners.contains_key(name)
    }

    pub fn contains_project(&self, owner: &Arc<str>, project: &Arc<str>) -> bool {
        self.projects.contains_key(&ProjectKey {
            owner: owner.clone(),
            project: project.clone(),
        })
    }

    pub fn contains_channel(
        &self,
        owner: &Arc<str>,
        project: &Arc<str>,
        channel: &Arc<str>,
    ) -> bool {
        self.channels.contains_key(&ChannelKey {
            owner: owner.clone(),
            project: project.clone(),
            channel: channel.clone(),
        })
    }

    /* ------------------------- Insert (Create/Update) ------------------------- */

    pub fn insert_owner(&mut self, name: Arc<str>, data: OwnerData) {
        self.owners.insert(name, data);
    }

    pub fn insert_project(
        &mut self,
        owner: &Arc<str>,
        project: Arc<str>,
        data: ProjectData,
    ) -> bool {
        if !self.owners.contains_key(owner) {
            return false; // Owner must exist
        }
        let key = ProjectKey {
            owner: owner.clone(),
            project,
        };
        self.projects.insert(key, data);
        true
    }

    pub fn insert_channel(
        &mut self,
        owner: &Arc<str>,
        project: &Arc<str>,
        channel: Arc<str>,
        data: ChannelData,
    ) -> bool {
        if !self.projects.contains_key(&ProjectKey {
            owner: owner.clone(),
            project: project.clone(),
        }) {
            return false; // Project must exist
        }
        let key = ChannelKey {
            owner: owner.clone(),
            project: project.clone(),
            channel,
        };
        self.channels.insert(key, data);
        true
    }

    /* --------------------------------- Delete --------------------------------- */

    pub fn delete_owner(&mut self, owner: &Arc<str>) -> bool {
        if self.owners.remove(owner).is_some() {
            self.projects.retain(|k, _| !Arc::ptr_eq(&k.owner, owner));
            self.channels.retain(|k, _| !Arc::ptr_eq(&k.owner, owner));
            true
        } else {
            false
        }
    }

    pub fn delete_project(&mut self, owner: &Arc<str>, project: &Arc<str>) -> bool {
        let key = ProjectKey {
            owner: owner.clone(),
            project: project.clone(),
        };
        if self.projects.remove(&key).is_some() {
            self.channels
                .retain(|k, _| !(Arc::ptr_eq(&k.owner, owner) && Arc::ptr_eq(&k.project, project)));
            true
        } else {
            false
        }
    }

    pub fn delete_channel(
        &mut self,
        owner: &Arc<str>,
        project: &Arc<str>,
        channel: &Arc<str>,
    ) -> bool {
        let key = ChannelKey {
            owner: owner.clone(),
            project: project.clone(),
            channel: channel.clone(),
        };
        self.channels.remove(&key).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hashbrown::HashSet;
    use std::sync::Arc;

    fn arc(s: &str) -> Arc<str> {
        Arc::from(s)
    }

    #[test]
    fn test_insert_and_get_owner() {
        let mut repo = RepoMap::default();
        let owner = arc("alice");

        repo.insert_owner(owner.clone(), OwnerData {});
        assert!(repo.contains_owner(&owner));
        assert!(repo.get_owner(&owner).is_some());
    }

    #[test]
    fn test_insert_project_requires_owner() {
        let mut repo = RepoMap::default();
        let owner = arc("bob");
        let project = arc("myproj");

        // Should fail because owner doesn't exist
        assert!(!repo.insert_project(&owner, project.clone(), ProjectData {}));

        // Insert owner, then project succeeds
        repo.insert_owner(owner.clone(), OwnerData {});
        assert!(repo.insert_project(&owner, project.clone(), ProjectData {}));
        assert!(repo.contains_project(&owner, &project));
    }

    #[test]
    fn test_insert_channel_requires_project() {
        let mut repo = RepoMap::default();
        let owner = arc("carol");
        let project = arc("proj1");
        let channel = arc("stable");

        // Should fail because project doesn't exist
        assert!(!repo.insert_channel(
            &owner,
            &project,
            channel.clone(),
            ChannelData {
                version: "1.0".into(),
                valid_assets: HashSet::new(),
            }
        ));

        // Insert owner and project first
        repo.insert_owner(owner.clone(), OwnerData {});
        repo.insert_project(&owner, project.clone(), ProjectData {});
        assert!(repo.insert_channel(
            &owner,
            &project,
            channel.clone(),
            ChannelData {
                version: "1.0".into(),
                valid_assets: HashSet::new(),
            }
        ));

        assert!(repo.contains_channel(&owner, &project, &channel));
        let ch = repo.get_channel(&owner, &project, &channel).unwrap();
        assert_eq!(ch.version, "1.0");
    }

    #[test]
    fn test_delete_owner_removes_projects_and_channels() {
        let mut repo = RepoMap::default();
        let owner = arc("dave");
        let project = arc("p1");
        let channel = arc("beta");

        repo.insert_owner(owner.clone(), OwnerData {});
        repo.insert_project(&owner, project.clone(), ProjectData {});
        repo.insert_channel(
            &owner,
            &project,
            channel.clone(),
            ChannelData {
                version: "0.1".into(),
                valid_assets: HashSet::new(),
            },
        );

        assert!(repo.delete_owner(&owner));
        assert!(!repo.contains_owner(&owner));
        assert!(!repo.contains_project(&owner, &project));
        assert!(!repo.contains_channel(&owner, &project, &channel));
    }

    #[test]
    fn test_delete_project_removes_channels() {
        let mut repo = RepoMap::default();
        let owner = arc("eve");
        let project = arc("projx");
        let channel = arc("dev");

        repo.insert_owner(owner.clone(), OwnerData {});
        repo.insert_project(&owner, project.clone(), ProjectData {});
        repo.insert_channel(
            &owner,
            &project,
            channel.clone(),
            ChannelData {
                version: "0.9".into(),
                valid_assets: HashSet::new(),
            },
        );

        assert!(repo.delete_project(&owner, &project));
        assert!(!repo.contains_project(&owner, &project));
        assert!(!repo.contains_channel(&owner, &project, &channel));
    }

    #[test]
    fn test_retain_channels_projects_owners() {
        let mut repo = RepoMap::default();

        let owner1 = arc("o1");
        let owner2 = arc("o2");
        let project1 = arc("p1");
        let project2 = arc("p2");
        let channel1 = arc("c1");
        let channel2 = arc("c2");

        repo.insert_owner(owner1.clone(), OwnerData {});
        repo.insert_owner(owner2.clone(), OwnerData {});

        repo.insert_project(&owner1, project1.clone(), ProjectData {});
        repo.insert_project(&owner2, project2.clone(), ProjectData {});

        repo.insert_channel(
            &owner1,
            &project1,
            channel1.clone(),
            ChannelData {
                version: "v1".into(),
                valid_assets: HashSet::new(),
            },
        );
        repo.insert_channel(
            &owner2,
            &project2,
            channel2.clone(),
            ChannelData {
                version: "v2".into(),
                valid_assets: HashSet::new(),
            },
        );

        // Retain only owner1
        repo.retain_owners(|o, _| o.as_ref() == "o1");
        assert!(repo.contains_owner(&owner1));
        assert!(!repo.contains_owner(&owner2));

        // Retain only project1
        repo.retain_projects(|o, p, _| o.as_ref() == "o1" && p.as_ref() == "p1");
        assert!(repo.contains_project(&owner1, &project1));
        assert!(!repo.contains_project(&owner2, &project2));

        // Retain only channel1
        repo.retain_channels(|o, p, c, _| {
            o.as_ref() == "o1" && p.as_ref() == "p1" && c.as_ref() == "c1"
        });
        assert!(repo.contains_channel(&owner1, &project1, &channel1));
        assert!(!repo.contains_channel(&owner2, &project2, &channel2));
    }

    #[test]
    fn test_channels_for_project() {
        let mut repo = RepoMap::default();
        let owner = arc("alice");
        let project = arc("proj");
        let channel1 = arc("c1");
        let channel2 = arc("c2");

        repo.insert_owner(owner.clone(), OwnerData {});
        repo.insert_project(&owner, project.clone(), ProjectData {});
        repo.insert_channel(
            &owner,
            &project,
            channel1.clone(),
            ChannelData {
                version: "v1".into(),
                valid_assets: HashSet::new(),
            },
        );
        repo.insert_channel(
            &owner,
            &project,
            channel2.clone(),
            ChannelData {
                version: "v2".into(),
                valid_assets: HashSet::new(),
            },
        );

        let channels: Vec<_> = repo.channels_for_project(&owner, &project).collect();
        assert_eq!(channels.len(), 2);
    }
}
