use std::{collections::HashMap, fmt::Display};

use crate::{
    project::{Project, ProjectOwner},
    provider::{
        forgejo::{owner::ForgejoProjectOwner, page::ForgejoPage},
        scanner::ChannelData,
    },
};

#[derive(Debug)]
pub enum ForgejoProjectError {}

impl Display for ForgejoProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("")
    }
}

impl std::error::Error for ForgejoProjectError {}

pub struct ForgejoProject<'a> {
    pub owner: &'a ForgejoProjectOwner,
    id: String,
}

impl<'a> ForgejoProject<'a> {
    pub fn new(owner: &'a ForgejoProjectOwner, id: String) -> Self {
        Self { owner, id }
    }
}

impl<'a> Project for ForgejoProject<'a> {
    type Page<'b>
        = ForgejoPage<'b>
    where
        Self: 'b;

    type Error = ForgejoProjectError;

    fn name(&self) -> &str {
        &self.id
    }

    async fn channels<'b>(&'b self) -> Result<impl Iterator<Item = Self::Page<'b>>, Self::Error> {
        let mut projects = HashMap::<String, ChannelData>::new();
        {
            let repos = self.owner.analyzer.data.repos.read().await;

            for (k, v) in repos.channels_for_project(self.owner.name(), &self.id) {
                projects.insert(k.to_string(), v.clone());
            }
        }

        // Initializing ForgejoProject may be heavy; lazy load (no collect)
        Ok(projects
            .into_iter()
            .map(move |(name, data)| ForgejoPage::new(self, name, data.version)))
    }

    fn default_channel(&self) -> Option<&str> {
        Some("pages")
    }
}
