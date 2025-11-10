//! Domain module
//!
//! Provides an interface for managing and resolving domains.

use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt::Display,
    path::{Path, PathBuf},
    sync::Arc,
};

use tokio::sync::RwLock;
use url::Url;

use crate::{
    event::{Event, EventBus},
    ext::Normalizable,
    resolution::{PageResolver, ResolutionError},
    upstream::{AssetLocation, PageLocation, Upstream},
};

/* -------------------------------------------------------------------------- */
/*                                   Errors                                   */
/* -------------------------------------------------------------------------- */

/// Occurs if a domain failed to resolve to a page
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageDomainError {
    /// More than one page is using this domain
    DomainCollision(u32),
    /// Something broke in the provider
    ProviderError,
    /// Nothing was found at that domain
    NotFound,
}

impl Display for PageDomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for PageDomainError {}

/* -------------------------------------------------------------------------- */
/*                                   Traits                                   */
/* -------------------------------------------------------------------------- */

/// Finds a page by a domain.
///
/// This helps with allowing custom domains for pages.
#[allow(async_fn_in_trait)]
pub trait PageDomainResolver {
    /// Resolves a given domain into a page
    async fn resolve_domain(&self, domain: &str) -> Result<Arc<dyn PageLocation>, PageDomainError>;

    /// Update the list of domains registered, if available, internally now
    async fn refresh(&self) -> Result<(), PageDomainError>;
}

/* -------------------------------------------------------------------------- */
/*                     Upstream-based Page Domain Resolver                    */
/* -------------------------------------------------------------------------- */

#[derive(Debug)]
struct DomainPageLocation {
    owner: Arc<str>,
    project: Arc<str>,
    channel: Arc<str>,
}

impl PageLocation for DomainPageLocation {
    fn owner(&self) -> &str {
        &self.owner
    }
    fn project(&self) -> &str {
        &self.project
    }
    fn channel(&self) -> &str {
        &self.channel
    }
}

#[derive(Debug)]
struct DomainAssetLocation {
    owner: String,
    project: String,
    channel: String,
    path: PathBuf,
}

impl PageLocation for DomainAssetLocation {
    fn owner(&self) -> &str {
        &self.owner
    }
    fn project(&self) -> &str {
        &self.project
    }
    fn channel(&self) -> &str {
        &self.channel
    }
}

impl AssetLocation for DomainAssetLocation {
    fn path(&self) -> &std::path::Path {
        &self.path
    }
}

#[derive(Default, Debug)]
struct ResolverData {
    domain_versions: HashMap<String, String>,
    domain_locations: HashMap<String, Arc<DomainPageLocation>>,
    colliding_domains: HashMap<String, u32>, // If present in here, multiple things have it, making it erroneous.
}

/// Finds pages by their domains via reading from an upstream
pub struct UpstreamPageDomainResolver<U: Upstream + 'static> {
    upstream: Arc<U>,
    data: Arc<RwLock<ResolverData>>,
}

impl<U: Upstream + 'static> UpstreamPageDomainResolver<U> {
    /// Creates a new UpstreamPageDomainResolver.
    ///
    /// The Upstream provided will be read from in order to determine pages and their domains.
    pub async fn new(upstream: Arc<U>, event_bus: EventBus) -> Self {
        let resolver = Self {
            upstream,
            data: Arc::new(RwLock::new(ResolverData::default())),
        };

        // Initial refresh
        resolver.refresh().await.unwrap_or_else(|e| {
            tracing::error!("Initial refresh failed: {:?}", e);
        });

        // Hook into the event bus
        let data = resolver.data.clone();
        let upstream = resolver.upstream.clone();
        event_bus.subscribe(move |event| {
            let data = data.clone();
            let upstream = upstream.clone();
            match event {
                Event::PageAvailable {
                    owner,
                    project,
                    channel,
                } => {
                    tokio::task::spawn(async move {
                        let data: Arc<RwLock<ResolverData>> = data.clone();
                        let upstream: Arc<U> = upstream.clone();
                        let version_key = format!("{}/{}/{}", owner, project, channel);

                        let mut data = data.write().await;
                        data.domain_versions.remove(&version_key);

                        let path = std::path::Path::new("/.domain");
                        if let Ok(bytes) = upstream
                            .get_asset_bytes(&owner, &project, &channel, path)
                            .await
                            && let Ok(body) = std::str::from_utf8(&bytes)
                        {
                            data.domain_locations.retain(|_, loc| {
                                !(loc.owner == owner
                                    && loc.project == project
                                    && loc.channel == channel)
                            });

                            for domain in body.lines().map(str::trim).filter(|l| !l.is_empty()) {
                                if data.domain_locations.contains_key(domain) {
                                    data.colliding_domains.insert(domain.to_string(), 2);
                                    tracing::warn!(
                                        "Domain collision detected for {} on update {}/{}/{}",
                                        domain,
                                        owner,
                                        project,
                                        channel
                                    );
                                } else {
                                    let loc = Arc::new(DomainPageLocation {
                                        owner: owner.clone(),
                                        project: project.clone(),
                                        channel: channel.clone(),
                                    });
                                    data.domain_locations.insert(domain.to_string(), loc);
                                    data.colliding_domains.remove(domain);
                                }
                            }

                            data.domain_versions
                                .insert(version_key, "updated_version".to_string());
                        }
                    });
                }

                Event::PageDeleted {
                    owner,
                    project,
                    channel,
                } => {
                    tokio::task::spawn(async move {
                        let mut data = data.write().await;
                        data.domain_locations.retain(|_, loc| {
                            !(loc.owner == owner
                                && loc.project == project
                                && loc.channel == channel)
                        });
                        let version_key = format!("{}/{}/{}", owner, project, channel);
                        data.domain_versions.remove(&version_key);
                        tracing::info!(
                            "Removed domains for deleted page {}/{}/{}",
                            owner,
                            project,
                            channel
                        );
                    });
                }
                #[allow(unreachable_patterns)]
                _ => {}
            }
        });

        resolver
    }
}

impl<U: Upstream> PageDomainResolver for UpstreamPageDomainResolver<U> {
    async fn resolve_domain(&self, domain: &str) -> Result<Arc<dyn PageLocation>, PageDomainError> {
        let r = self.data.read().await;
        if let Some(count) = r.colliding_domains.get(domain) {
            return Err(PageDomainError::DomainCollision(*count));
        }

        match r.domain_locations.get(domain) {
            Some(l) => Ok(l.clone()),
            None => Err(PageDomainError::NotFound),
        }
    }

    async fn refresh(&self) -> Result<(), PageDomainError> {
        let owners = self.upstream.list_owners().await.map_err(|e| {
            tracing::error!("Failed to list owners: {:?}", e);
            PageDomainError::ProviderError
        })?;

        let mut data = self.data.write().await;

        let mut new_domain_versions = HashMap::new();

        let mut domain_locations_map: HashMap<String, Vec<Arc<DomainPageLocation>>> =
            HashMap::new();

        for owner in owners.iter() {
            let projects = match self.upstream.list_projects(owner).await {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("Failed to list projects for {}: {:?}", owner, e);
                    continue;
                }
            };

            for project in projects.iter() {
                let channels = match self.upstream.list_channels(owner, project).await {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!(
                            "Failed to list channels for {}/{}: {:?}",
                            owner,
                            project,
                            e
                        );
                        continue;
                    }
                };

                for channel in channels.iter() {
                    let version = match self
                        .upstream
                        .get_page_version(owner, project, channel)
                        .await
                    {
                        Ok(v) => v,
                        Err(e) => {
                            tracing::error!(
                                "Failed to get version for {}/{}/{}: {:?}",
                                owner,
                                project,
                                channel,
                                e
                            );
                            continue;
                        }
                    };

                    let version_key = format!("{}/{}/{}", owner, project, channel);
                    new_domain_versions.insert(version_key.clone(), version.clone());

                    // Always fetch domains, even if the version hasn't changed
                    let bytes = match self
                        .upstream
                        .get_asset_bytes(owner, project, channel, Path::new("/.domain"))
                        .await
                    {
                        Ok(b) => b,
                        Err(_) => continue,
                    };
                    let body = match std::str::from_utf8(&bytes) {
                        Ok(s) => s,
                        Err(_) => continue,
                    };

                    for domain in body.lines().map(str::trim).filter(|l| !l.is_empty()) {
                        let loc = Arc::new(DomainPageLocation {
                            owner: Arc::from(owner.to_string()),
                            project: Arc::from(project.to_string()),
                            channel: Arc::from(channel.to_string()),
                        });
                        domain_locations_map
                            .entry(domain.to_string())
                            .or_default()
                            .push(loc);
                    }
                }
            }
        }

        // Compute collisions and canonical locations
        let mut new_domain_locations = HashMap::new();
        let mut colliding_domains = HashMap::new();

        for (domain, locs) in domain_locations_map {
            let mut unique_locs = HashSet::new();
            for loc in locs.iter() {
                unique_locs.insert((loc.owner(), loc.project(), loc.channel()));
            }
            if unique_locs.len() > 1 {
                colliding_domains.insert(domain.clone(), unique_locs.len() as u32);
                tracing::warn!(
                    "Domain collision detected for {}: {} locations",
                    domain,
                    unique_locs.len()
                );
            }
            if let Some(first_loc) = locs.first() {
                new_domain_locations.insert(domain, Arc::clone(first_loc));
            }
        }

        // Replace old data with new refreshed state
        data.domain_locations = new_domain_locations;
        data.colliding_domains = colliding_domains;
        data.domain_versions = new_domain_versions;

        tracing::info!("Domain resolver refresh complete");
        Ok(())
    }
}

impl<U: Upstream> PageResolver for UpstreamPageDomainResolver<U> {
    async fn resolve_url(&self, url: &Url) -> Result<Arc<dyn AssetLocation>, ResolutionError> {
        let host = match url.domain() {
            Some(host) => host,
            None => {
                return Err(ResolutionError::Invalid(""));
            }
        };

        self.resolve_domain(host)
            .await
            .map(|f| {
                let path = Path::new(url.path()).normalized_relative();

                let asset: Arc<dyn AssetLocation> = Arc::new(DomainAssetLocation {
                    owner: f.owner().to_string(),
                    project: f.project().to_string(),
                    channel: f.channel().to_string(),
                    path,
                });
                asset
            })
            .map_err(|_| ResolutionError::ProviderError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::upstream::mock::MockUpstream;
    use std::path::Path;

    const OWNER1: &str = "owner1";
    const PROJECT1: &str = "project1";
    const CHANNEL1: &str = "main";
    const DOMAIN1: &str = "example.com";

    const OWNER2: &str = "owner2";
    const PROJECT2: &str = "project2";
    const CHANNEL2: &str = "dev";
    const DOMAIN2: &str = "other.com";

    const DOMAIN_PATH: &str = "/.domain";

    #[tokio::test]
    async fn resolves_existing_domain_via_refresh() {
        let path = Path::new(DOMAIN_PATH);
        let upstream = Arc::new(MockUpstream::default().with_asset(
            OWNER1,
            PROJECT1,
            CHANNEL1,
            path,
            DOMAIN1.as_bytes(),
        ));

        let resolver = UpstreamPageDomainResolver::new(upstream, EventBus::default()).await;
        //       resolver.refresh().await.unwrap();

        let resolved = resolver.resolve_domain(DOMAIN1).await.unwrap();
        assert_eq!(resolved.owner(), OWNER1);
        assert_eq!(resolved.project(), PROJECT1);
        assert_eq!(resolved.channel(), CHANNEL1);
    }

    #[tokio::test]
    async fn returns_not_found_for_missing_domain() {
        let upstream = Arc::new(MockUpstream::default()); // No domains
        let resolver = UpstreamPageDomainResolver::new(upstream, EventBus::default()).await;
        resolver.refresh().await.unwrap();

        let err = resolver.resolve_domain("missing.com").await.unwrap_err();
        assert_eq!(err, PageDomainError::NotFound);
    }

    #[tokio::test]
    async fn detects_domain_collisions() {
        let path = Path::new(DOMAIN_PATH);
        // Two owners/projects/channels claiming the same domain
        let upstream = Arc::new(
            MockUpstream::default()
                .with_asset(OWNER1, PROJECT1, CHANNEL1, path, DOMAIN1.as_bytes())
                .with_asset(OWNER2, PROJECT2, CHANNEL2, path, DOMAIN1.as_bytes()),
        ); // collision

        let resolver = UpstreamPageDomainResolver::new(upstream, EventBus::default()).await;

        let err = resolver.resolve_domain(DOMAIN1).await.unwrap_err();
        match err {
            PageDomainError::DomainCollision(count) => {
                assert_eq!(count, 2);
            } // Currently returns ProviderError for collisions
            _ => panic!("Expected collision error"),
        }

        // After a second refresh
        resolver.refresh().await.unwrap();

        let err = resolver.resolve_domain(DOMAIN1).await.unwrap_err();
        match err {
            PageDomainError::DomainCollision(count) => {
                assert_eq!(count, 2);
            } // Currently returns ProviderError for collisions
            _ => panic!("Expected collision error"),
        }
    }

    #[tokio::test]
    async fn resolves_multiple_domains_correctly() {
        let path = Path::new(DOMAIN_PATH);
        let upstream = Arc::new(
            MockUpstream::default()
                .with_asset(OWNER1, PROJECT1, CHANNEL1, path, DOMAIN1.as_bytes())
                .with_asset(OWNER2, PROJECT2, CHANNEL2, path, DOMAIN2.as_bytes()),
        );

        let resolver = UpstreamPageDomainResolver::new(upstream, EventBus::default()).await;
        resolver.refresh().await.unwrap();

        let resolved1 = resolver.resolve_domain(DOMAIN1).await.unwrap();
        assert_eq!(resolved1.owner(), OWNER1);

        let resolved2 = resolver.resolve_domain(DOMAIN2).await.unwrap();
        assert_eq!(resolved2.owner(), OWNER2);
    }

    #[tokio::test]
    async fn ignores_empty_or_whitespace_domains() {
        let path = Path::new(DOMAIN_PATH);
        let upstream = Arc::new(MockUpstream::default().with_asset(
            OWNER1,
            PROJECT1,
            CHANNEL1,
            path,
            b" \n\t\nexample.com\n",
        ));

        let resolver = UpstreamPageDomainResolver::new(upstream, EventBus::default()).await;
        resolver.refresh().await.unwrap();

        let resolved = resolver.resolve_domain("example.com").await.unwrap();
        assert_eq!(resolved.owner(), OWNER1);

        // The whitespace lines should not create any invalid domains
        let err = resolver.resolve_domain("").await.unwrap_err();
        assert_eq!(err, PageDomainError::NotFound);
    }
}
