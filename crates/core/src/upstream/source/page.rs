use std::{process::Output, sync::Arc};

use crate::upstream::UpstreamError;

/// Can get the latest available version of a page.
pub trait PageVersionSource: Send + Sync {
    /// Returns the current version or revision of a given page.
    ///
    /// This can be used to perform cache invalidation.
    ///
    /// # Errors
    ///
    /// - `NotFound` - The page or asset was not available.
    /// - `ProviderError` - Something happened in the upstream that caused a failure.
    #[allow(unused_variables)]
    fn get_page_version(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> impl Future<Output = Result<String, UpstreamError>> + Send;
}

/// Can get a list of all pages, projects, and owners in the latest version of a page.
pub trait PageListComponentsSource: Send + Sync {
    /// List all project owners available.
    ///
    /// # Errors
    ///
    /// - `ProviderError` - Something happened in the upstream that caused a failure.
    /// - `NotImplemented` - The upstream is not capable of performing this.
    fn list_owners(&self) -> impl Future<Output = Result<Arc<[String]>, UpstreamError>> + Send {
        async move { Err(UpstreamError::NotImplemented) }
    }

    /// List all projects available to a specific owner.
    ///
    /// # Errors
    ///
    /// - `NotFound` - The owner wasn't available.
    /// - `ProviderError` - Something happened in the upstream that caused a failure.
    /// - `NotImplemented` - The upstream is not capable of performing this.
    #[allow(unused_variables)]
    fn list_projects(
        &self,
        owner: &str,
    ) -> impl Future<Output = Result<Arc<[String]>, UpstreamError>> + Send {
        async move { Err(UpstreamError::NotImplemented) }
    }

    /// List all channels available in a specific project.
    ///
    /// # Errors
    ///
    /// - `NotFound` - The project wasn't available.
    /// - `ProviderError` - Something happened in the upstream that caused a failure.
    /// - `NotImplemented` - The upstream is not capable of performing this.
    #[allow(unused_variables)]
    fn list_channels(
        &self,
        owner: &str,
        project: &str,
    ) -> impl Future<Output = Result<Arc<[String]>, UpstreamError>> + Send {
        async move { Err(UpstreamError::NotImplemented) }
    }

    /// Returns if a given page is available.
    ///
    /// # Errors
    ///
    /// - `ProviderError` - Something happened in the upstream that caused a failure.
    fn has_page(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
    ) -> impl Future<Output = Result<bool, UpstreamError>> + Send {
        async move {
            match self.list_channels(owner, project).await {
                Ok(chans) => Ok(chans.iter().any(|c| c == channel)),
                Err(UpstreamError::NotImplemented) => Ok(false),
                Err(e) => Err(e),
            }
        }
    }
}

pub trait PageListSource: Send + Sync {
    /// List all pages available in (owner, project, channel) format.
    ///
    /// # Errors
    ///
    /// - `ProviderError` - Something happened in the upstream that caused a failure.
    /// - `NotImplemented` - The upstream is not capable of performing this.
    // TODO: Weird return type? Probably not very compact...
    fn list_pages(
        &self,
    ) -> impl Future<
        Output = Result<Arc<(Arc<[String]>, Arc<[String]>, Arc<[String]>)>, UpstreamError>,
    > + Send;
}
