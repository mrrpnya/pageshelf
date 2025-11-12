//! Page resolution module
//!
//! Provides an interface for page searching (via URL)

use std::{path::Path, sync::Arc};

use url::Url;
mod directory;
mod regex;
mod subdomain;
use crate::{
    domain::UpstreamPageDomainResolver,
    event::EventBus,
    upstream::{AssetLocation, Upstream, source::PageListSource},
};
pub use directory::DirectoryPageResolver;
pub use regex::{RegexPageFilter, RegexPageFilterRules};
pub use subdomain::SubdomainPageResolver;
/* -------------------------------------------------------------------------- */
/*                                   Errors                                   */
/* -------------------------------------------------------------------------- */

/// Occurs when something failed to resolve into a URL
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionError {
    /// The request may have been valid, but was denied by access control
    Unauthorized(&'static str),
    /// The request was not valid
    Invalid(&'static str),
    /// Something broke inside the provider
    ProviderError,
}

/* -------------------------------------------------------------------------- */
/*                                   Default                                  */
/* -------------------------------------------------------------------------- */

/// Creates a page resolver setup with reasonable defaults and a domain resolver.
///
/// Additionally refreshes said domain resolver off the bat, hence the `async`.
pub async fn default_page_resolver<U: Upstream + PageListSource + 'static>(
    upstream: Arc<U>,
    subdomain_domains: Vec<String>,
    event_bus: EventBus,
) -> impl PageResolver {
    DirectoryPageResolver::new(Some("pages".to_string()), Some("pages".to_string()))
        .with_layer(SubdomainPageResolver::new(
            Some(subdomain_domains),
            Some("pages".to_string()),
            Some("pages".to_string()),
        ))
        .with_layer(UpstreamPageDomainResolver::new(upstream, event_bus).await)
}

/* -------------------------------------------------------------------------- */
/*                                   Traits                                   */
/* -------------------------------------------------------------------------- */

/// Resolves a URL to the location of a page and an asset within
// TODO: Make PageResolver and associated traits generic perhaps?
#[allow(async_fn_in_trait)]
pub trait PageResolver: Sized {
    /// Resolves a URL to the location of an asset inside a page.
    ///
    /// Returns an [AssetLocation] that represents the location.
    async fn resolve_url(&self, url: &Url) -> Result<Arc<dyn AssetLocation>, ResolutionError>;

    /// Applies a [PageFilter] atop this resolver to control access.
    ///
    /// Queries will have to go be checked by the filter before going to this resolver,
    /// and locations will be checked before being returned.
    fn with_filter<F: PageFilter>(self, filter: F) -> impl PageResolver {
        FilteredPageResolverLayer {
            inner: self,
            filter,
        }
    }
    /// Applies another [PageResolver] atop this resolver, which will take priority
    ///
    /// The provided resolver will attempt first, failing that, this resolver will be attempted next.
    fn with_layer<R: PageResolver>(self, primary: R) -> LayeredPageResolver<R, Self> {
        LayeredPageResolver {
            primary,
            fallback: self,
        }
    }
}

/* ------------------------------- Filtration ------------------------------- */

/// Allows for implementing access control on top of a [PageResolver].
pub trait PageFilter {
    /// Checks if a given URL is allowed.
    ///
    /// This should be checked *before* the page is resolved
    #[allow(unused_variables)]
    fn allow_url(&self, url: &Url) -> bool {
        true
    }

    /// Checks if a given owner of a project is allowed.
    ///
    /// This should be checked *after* the page is resolved
    #[allow(unused_variables)]
    fn allow_owner(&self, owner: &str) -> bool {
        true
    }

    /// Checks if a given project is allowed.
    ///
    /// This should be checked *after* the page is resolved
    #[allow(unused_variables)]
    fn allow_project(&self, project: &str) -> bool {
        true
    }

    /// Checks if a given channel of a project is allowed.
    ///
    /// This should be checked *after* the page is resolved
    #[allow(unused_variables)]
    fn allow_channel(&self, channel: &str) -> bool {
        true
    }

    /// Checks if a given asset is allowed.
    ///
    /// This should be checked *after* the page is resolved
    #[allow(unused_variables)]
    fn allow_asset(&self, asset: &Path) -> bool {
        true
    }
}

/* -------------------------------------------------------------------------- */
/*                                Filter helper                               */
/* -------------------------------------------------------------------------- */

/// A helper meant to aid in adding access control to a given [PageResolver].
///
/// This intercepts both the request and the result in order to perform checks with a [PageFilter].
/// This can also be stacked.
struct FilteredPageResolverLayer<R: PageResolver, F: PageFilter> {
    inner: R,
    filter: F,
}

impl<R: PageResolver, F: PageFilter> PageResolver for FilteredPageResolverLayer<R, F> {
    async fn resolve_url(&self, url: &Url) -> Result<Arc<dyn AssetLocation>, ResolutionError> {
        if !self.filter.allow_url(url) {
            return Err(ResolutionError::Unauthorized("URL not allowed"));
        }

        let asset = self.inner.resolve_url(url).await?;

        if !self.filter.allow_owner(asset.owner()) {
            return Err(ResolutionError::Unauthorized("Owner not allowed"));
        }
        if !self.filter.allow_project(asset.project()) {
            return Err(ResolutionError::Unauthorized("Project not allowed"));
        }
        if !self.filter.allow_channel(asset.channel()) {
            return Err(ResolutionError::Unauthorized("Channel not allowed"));
        }
        if !self.filter.allow_asset(asset.path()) {
            return Err(ResolutionError::Unauthorized("Asset not allowed"));
        }

        Ok(asset)
    }
}

/* -------------------------------------------------------------------------- */
/*                               Layer system                                 */
/* -------------------------------------------------------------------------- */

/// A helper struct to allow merging multiple [PageResolver]s in a priority.
///
/// If the primary resolver fails, the fallback will be tried instead.
pub struct LayeredPageResolver<PRIMARY: PageResolver, FALLBACK: PageResolver> {
    primary: PRIMARY,
    fallback: FALLBACK,
}

impl<R1, R2> PageResolver for LayeredPageResolver<R1, R2>
where
    R1: PageResolver + Send + Sync,
    R2: PageResolver + Send + Sync,
{
    async fn resolve_url(&self, url: &Url) -> Result<Arc<dyn AssetLocation>, ResolutionError> {
        match self.primary.resolve_url(url).await {
            Ok(asset) => Ok(asset),
            Err(_) => self.fallback.resolve_url(url).await,
        }
    }
}
