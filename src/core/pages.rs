//! Deals with the utilities for loading pages.
//! Generally, to access something a page, you go through these steps:
//! PageSource -> Page -> Asset -> [your data]

#[cfg(feature = "forgejo")]
use crate::{Asset, AssetSource};
use log::{error, info};
use std::{fmt::Display, path::Path};

/* -------------------------------- Constants ------------------------------- */

// TODO: Allow changing behavior regarding handing of domain files

/// The relative location in which to find page domain configuration.
pub const DOMAIN_FILE_PATH: &str = "/.domain";

/* -------------------------------- Utilities ------------------------------- */

#[derive(Debug, PartialEq, Eq)]
pub enum PageError {
    /// The desired page wasn't found.
    NotFound,
    /// Something went wrong in the Page Provider.
    ProviderError,
}

/// Allows displaying Page Errors in a human readable format
impl Display for PageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => f.write_str("Not found"),
            Self::ProviderError => f.write_str("Provider error"),
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                               Page Accessing                               */
/* -------------------------------------------------------------------------- */

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageLocation {
    pub owner: String,
    pub name: String,
    pub branch: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageAssetLocation {
    pub page: PageLocation,
    pub asset: String,
}

/// A Page represents a specific site to be hosted.
pub trait Page: AssetSource {
    fn name(&self) -> &str;
    /// Branch of the Page - This allows pages to have variants.
    /// This can allow you to have your main page at `pages`, but a testing page at `pages-testing`,
    /// and they can be individually addressed.
    fn branch(&self) -> &str;
    fn owner(&self) -> &str;
    fn location(&self) -> PageLocation {
        PageLocation {
            owner: self.owner().to_string(),
            name: self.name().to_string(),
            branch: self.branch().to_string(),
        }
    }
    fn version(&self) -> &str;
}

/* -------------------------------------------------------------------------- */
/*                                Page Sourcing                               */
/* -------------------------------------------------------------------------- */

/* -------------------------------- Querying -------------------------------- */

/// A query that allows you to find pages that meet certain criteria.
#[derive(Debug)]
pub struct PageQuery<'a> {
    // TODO: Consider using dynamic parameters for finer control
    // Using no dynamic stuff, only references right now to prevent allocations
    /// If anyone, who should own the page?
    owner: Option<&'a [&'a str]>,
    /// If any, what should the page be named?
    name: Option<&'a [&'a str]>,
    /// If any, what branch should the page be?
    branch: Option<&'a [&'a str]>,
}

/* -------------------------------- Sourcing -------------------------------- */

impl<'a> PageQuery<'a> {
    /// Creates a simple query that will find anything
    pub fn anything() -> Self {
        Self {
            owner: None,
            name: None,
            branch: None,
        }
    }

    /* --------------------------------- Factory -------------------------------- */

    /// Factory function to require certain owners on this query
    pub fn with_owners(mut self, owners: &'a [&'a str]) -> Self {
        self.branch = Some(owners);
        self
    }

    /// Factory function to require certain names on this query
    pub fn with_names(mut self, names: &'a [&'a str]) -> Self {
        self.branch = Some(names);
        self
    }

    /// Factory function to require certain names on this query
    pub fn with_branches(mut self, branches: &'a [&'a str]) -> Self {
        self.branch = Some(branches);
        self
    }
}

impl<'a> Default for PageQuery<'a> {
    fn default() -> Self {
        Self::anything()
    }
}

/// You can find Pages in a Page Source.
pub trait PageSource {
    // TODO: Move these away from future and to async once the compiler no longer warns
    // (They function the same at least)

    /// Tries to get a Page at the specified location.
    fn page_at(
        &self,
        owner: String,
        name: String,
        branch: String,
    ) -> impl Future<Output = Result<impl Page, PageError>>;
    /// Iterates all pages available to this source.
    fn pages(&self) -> impl Future<Output = Result<impl Iterator<Item = impl Page>, PageError>>;

    /// What branch should be inferred when there is no specified branch?
    fn default_branch(&self) -> &str {
        "pages"
    }

    /* ------------------------- Automatic Abstractions ------------------------- */

    /// Find all Pages that meet conditions set by the query
    #[allow(async_fn_in_trait)]
    async fn search_pages<'a>(
        &self,
        query: &PageQuery<'a>,
    ) -> Result<impl Iterator<Item = impl Page>, PageError> {
        match self.pages().await {
            Ok(v) => {
                Ok(v.filter(|page| {
                    // TODO: Consider changing this from simple match to regex?
                    // Owner check
                    if let Some(v) = &query.owner {
                        let owner = page.owner();
                        return v.iter().any(|f| f == &owner);
                    }
                    // Name check
                    if let Some(v) = &query.name {
                        let name = page.name();
                        return v.iter().any(|f| f == &name);
                    }
                    // Name check
                    if let Some(v) = &query.branch {
                        let branch = page.name();
                        return v.iter().any(|f| f == &branch);
                    }

                    true
                }))
            }
            Err(e) => {
                error!("Error searching for page (query: {:?}): {}", query, e);
                Err(PageError::ProviderError)
            }
        }
    }

    #[allow(async_fn_in_trait)]
    async fn branches_used<'a>(
        &self,
        query: &PageQuery<'a>,
    ) -> Result<impl Iterator<Item = String>, PageError> {
        match self.search_pages(query).await {
            Ok(pages) => Ok(pages.map(|f| f.branch().to_string())),
            Err(e) => {
                error!(
                    "Error when finding what branches were being used (query: {:?}): {}",
                    query, e
                );
                Err(e)
            }
        }
    }

    #[allow(async_fn_in_trait)]
    async fn find_by_domains(&self, domains: &[&str]) -> Result<impl Page, PageError> {
        let pages = self.pages().await;
        if let Err(e) = pages {
            error!("Error getting pages to find: {}", e);
            return Err(e);
        }
        let pages = pages.unwrap();
        for page in pages {
            let mut applies = false;
            {
                // TODO: Magic string, fix.
                info!(
                    "Checking repo {}/{}:{} for domain file. Matching against domains {:?}...",
                    page.owner(),
                    page.name(),
                    page.branch(),
                    domains
                );
                let asset = page.get_asset(Path::new(DOMAIN_FILE_PATH)).await;

                if let Ok(asset) = asset {
                    info!(
                        "Found domain file at {}/{}:{}",
                        page.owner(),
                        page.name(),
                        page.branch()
                    );
                    let bytes = asset.bytes();
                    if let Ok(body) = std::str::from_utf8(bytes) {
                        // Trim lines in the body to avoid whitespace issues
                        let trimmed_body_lines: Vec<String> = body
                            .split('\n')
                            .map(|line| line.trim().to_string())
                            .collect();

                        // Check if any domain is in the trimmed body lines
                        if trimmed_body_lines
                            .iter()
                            .any(|line| domains.contains(&line.as_str()))
                        {
                            applies = true;
                        }
                    }
                }
            }
            if applies {
                info!("Resolved page");
                return Ok(page);
            }
        }

        Err(PageError::NotFound)
    }
}

/* -------------------------------------------------------------------------- */
/*                                    Tests                                   */
/* -------------------------------------------------------------------------- */
