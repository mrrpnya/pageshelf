use std::{collections::HashMap, marker::PhantomData, sync::Arc};

use forgejo_api::Forgejo;

use crate::{
    project::ProjectOwner,
    provider::forgejo::{project::ForgejoProject, scanner::ForgejoScanner},
};

pub struct ForgejoProjectOwner {
    id: String,
    pub forgejo: Arc<Forgejo>,
    pub analyzer: Arc<ForgejoScanner>,
}

impl ForgejoProjectOwner {
    pub fn new(id: String, forgejo: Arc<Forgejo>, analyzer: Arc<ForgejoScanner>) -> Self {
        Self {
            id,
            analyzer,
            forgejo,
        }
    }
}

impl ProjectOwner for ForgejoProjectOwner {
    type Project<'a> = ForgejoProject<'a>;

    #[inline]
    fn name(&self) -> &str {
        &self.id
    }

    async fn projects<'a>(
        &'a self,
    ) -> Result<impl Iterator<Item = Self::Project<'a>>, crate::project::ProjectError> {
        let repos = self.analyzer.data.repos.read().await;
        let mut projects = HashMap::<String, ()>::new();
        {
            for (k, _) in repos.projects_for_owner(&self.id) {
                projects.insert(k.to_string(), ());
            }
        }

        // Initializing ForgejoProject may be heavy; lazy load (no collect)
        Ok(projects
            .into_keys()
            .map(move |name| ForgejoProject::new(self, name)))
    }

    async fn get_project<'a>(
        &'a self,
        name: &str,
    ) -> Result<Option<Self::Project<'a>>, crate::project::ProjectError> {
        let project: bool;
        {
            let repos = self.analyzer.data.repos.read().await;
            project = repos.contains_project(&self.id, name);
        }

        match project {
            true => Ok(Some(ForgejoProject::new(self, name.to_string()))),
            false => Ok(None),
        }
    }
}
