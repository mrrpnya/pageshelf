use crate::AssetError;
use crate::project::{DOMAIN_FILE_PATH, Page, Project, ProjectError, ProjectOwner};
use futures::stream::FuturesUnordered;
use futures::{Stream, StreamExt, TryStreamExt, stream};
use tracing::{error, info};
// TODO: Commenting pass

pub struct DomainPageResolution {
    pub owner: String,
    pub project: String,
    pub channel: String,
}

pub trait ProjectSource {
    type Owner<'a>: ProjectOwner + 'a
    where
        Self: 'a;

    async fn get_owner<'a>(&'a self, name: &str) -> Result<Option<Self::Owner<'a>>, ProjectError> {
        self.all_owners()
            .await
            .map(|i| i.filter(|f| f.name() == name).next())
    }

    async fn all_owners<'a>(
        &'a self,
    ) -> Result<impl Iterator<Item = Self::Owner<'a>>, ProjectError>;

    async fn resolve_domain<'a>(
        &'a self,
        domain: &str,
    ) -> Result<impl Iterator<Item = DomainPageResolution> + 'a, ProjectError> {
        // Step 1. Fetch all owners
        let owners = self.all_owners().await?;

        // Step 2. Create a stream of concurrent tasks
        let tasks = owners.into_iter().map(|owner| async move {
            let mut results = Vec::<DomainPageResolution>::new();

            let projects = match owner.projects().await {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!("Error resolving domain: {} when searching for projects", e);
                    return results;
                }
            };

            for project in projects {
                let channels = match project.channels().await {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::error!(
                            "Error resolving domain: {} when searching for channels",
                            e
                        );
                        return results;
                    }
                };
                for channel in channels {
                    match channel.domains().await {
                        Ok(mut channel_domains) => {
                            if channel_domains.any(|f| f == domain) {
                                results.push(DomainPageResolution {
                                    owner: owner.name().to_string(),
                                    project: project.name().to_string(),
                                    channel: channel.name().to_string(),
                                });
                            }
                        }
                        Err(AssetError::NotFound) => {}
                        Err(_) => {}
                    }
                }
            }

            results
        });

        // Step 3. Convert to FuturesUnordered to run concurrently
        let concurrent = FuturesUnordered::from_iter(tasks);
        let all_results: Vec<DomainPageResolution> = concurrent
            .collect::<Vec<_>>() // Vec<Vec<DomainPageResolution>>
            .await
            .into_iter()
            .flatten()
            .collect();

        // Step 4. Flatten results as they complete
        Ok(all_results.into_iter())
    }

    async fn has_page(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> Result<bool, ProjectError> {
        if let Some(owner) = self.get_owner(owner).await?
            && let Some(project) = owner.get_project(project).await?
            && let Ok(Some(_)) = project.get_channel(channel).await
        {
            return Ok(true);
        }
        Ok(false)
    }
}
