#[cfg(feature = "forgejo")]
use crate::asset::{Asset, AssetQueryable};
use log::{error, info};
/// Deals with the utilities for loading pages.
/// Generally, to access something a page, you go through these steps:
/// PageSource -> Page -> Asset -> [your data]
use std::{fmt::Display, path::Path};

/* -------------------------------- Constants ------------------------------- */

const FILE_DOMAIN: &str = "/.domain";

/* -------------------------------- Utilities ------------------------------- */

#[derive(Debug, PartialEq, Eq)]
pub enum PageError {
    /// The resource wasn't found.
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

/// A Page represents a site to be hosted.
pub trait Page: AssetQueryable {
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

/* --------------------------- Matching Utilities --------------------------- */

/// Identifies how we should see if a string matches a pattern
#[derive(Debug, PartialEq, Eq)]
pub enum StringMatchingType {
    /// If it just matches the pattern with simple comparison
    Simple,
}

impl StringMatchingType {
    /// Checks if a string matches a pattern.
    ///
    /// Arguments:
    /// - `pattern`: What pattern to check for a match using
    /// - `s`: The string to see if a match is present
    pub fn matches(&self, pattern: &str, s: &str) -> bool {
        match self {
            Self::Simple => pattern == s,
        }
    }
}

impl Default for StringMatchingType {
    fn default() -> Self {
        Self::Simple
    }
}

/* -------------------------------- Querying -------------------------------- */

#[derive(Debug)]
struct MatchingQueryField<T> {
    matcher: StringMatchingType,
    data: T,
}

impl<T> MatchingQueryField<T> {
    pub fn new(data: T, matcher: StringMatchingType) -> Self {
        Self { matcher, data }
    }

    pub fn data(&self) -> &T {
        &self.data
    }
}

/// A query that allows you to find pages that meet certain criteria.
#[derive(Debug)]
pub struct PageSourceQuery<'a> {
    // TODO: Consider using dynamic parameters for finer control
    // Using no dynamic stuff, only references right now to prevent allocations
    /// If anyone, who should own the page?
    owner: Option<MatchingQueryField<&'a [&'a str]>>,
    /// If any, what should the page be named?
    name: Option<MatchingQueryField<&'a [&'a str]>>,
    /// If any, what branch should the page be?
    branch: Option<MatchingQueryField<&'a [&'a str]>>,
}

/* -------------------------------- Sourcing -------------------------------- */

impl<'a> PageSourceQuery<'a> {
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
    pub fn with_owners(mut self, owners: &'a [&'a str], matcher: StringMatchingType) -> Self {
        self.branch = Some(MatchingQueryField::new(owners, matcher));
        self
    }

    /// Factory function to require certain names on this query
    pub fn with_names(mut self, names: &'a [&'a str], matcher: StringMatchingType) -> Self {
        self.branch = Some(MatchingQueryField::new(names, matcher));
        self
    }

    /// Factory function to require certain names on this query
    pub fn with_branches(mut self, branches: &'a [&'a str], matcher: StringMatchingType) -> Self {
        self.branch = Some(MatchingQueryField::new(branches, matcher));
        self
    }

    /* -------------------------------- Checking -------------------------------- */

    pub fn check_owner(&self, owner: &str) -> bool {
        match &self.owner {
            Some(v) => v.data.iter().any(|f| *f == owner),
            None => true,
        }
    }

    pub fn check_name(&self, name: &str) -> bool {
        match &self.name {
            Some(v) => v.data.iter().any(|f| *f == name),
            None => true,
        }
    }

    pub fn check_branch(&self, branch: &str) -> bool {
        match &self.branch {
            Some(v) => v.data.iter().any(|f| *f == branch),
            None => true,
        }
    }
}

impl<'a> Default for PageSourceQuery<'a> {
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
    async fn search_pages<'a>(
        &self,
        query: &PageSourceQuery<'a>,
    ) -> Result<impl Iterator<Item = impl Page>, PageError> {
        match self.pages().await {
            Ok(v) => {
                Ok(v.filter(|page| {
                    // TODO: Consider changing this from simple match to regex?
                    // Owner check
                    match &query.owner {
                        Some(v) => {
                            let owner = page.owner();
                            return v.data().iter().any(|f| f == &owner);
                        }
                        None => {}
                    }
                    // Name check
                    match &query.name {
                        Some(v) => {
                            let name = page.name();
                            return v.data().iter().any(|f| f == &name);
                        }
                        None => {}
                    }
                    // Name check
                    match &query.branch {
                        Some(v) => {
                            let branch = page.name();
                            return v.data().iter().any(|f| f == &branch);
                        }
                        None => {}
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

    async fn branches_used<'a>(
        &self,
        query: &PageSourceQuery<'a>,
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
                let asset = page.asset_at(Path::new(FILE_DOMAIN)).await;

                if let Ok(asset) = asset {
                    info!(
                        "Found domain file at {}/{}:{}",
                        page.owner(),
                        page.name(),
                        page.branch()
                    );
                    let body = asset.body();

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
            if applies {
                info!("Resolved page");
                return Ok(page);
            }
        }

        Err(PageError::NotFound)
    }
}

/* -------------------------------------------------------------------------- */
/*                             Page Source Factory                            */
/* -------------------------------------------------------------------------- */

/// Offers an impl-agnostic of creating Page Sources.
pub trait PageSourceFactory: Clone {
    type Source: PageSource;

    fn wrap<L: PageSourceLayer<Self::Source>>(self, layer: L) -> PageSourceFactoryLayer<Self, L> {
        PageSourceFactoryLayer {
            parent: self,
            layer,
        }
    }

    fn build(&self) -> Result<Self::Source, ()>;
}

/// Layers over a Page Source and can modify it.
/// You could, for instance, create a blacklist that won't accept certain queries.
pub trait PageSourceLayer<PS: PageSource>: Clone {
    type Source: PageSource;

    fn wrap(&self, page_source: PS) -> Self::Source;
}

#[derive(Clone)]
pub struct PageSourceFactoryLayer<F: PageSourceFactory, L: PageSourceLayer<F::Source>> {
    parent: F,
    layer: L,
}

impl<'a, F: PageSourceFactory, L: PageSourceLayer<F::Source>> PageSourceFactory
    for PageSourceFactoryLayer<F, L>
{
    type Source = L::Source;

    fn build(&self) -> Result<Self::Source, ()> {
        let built = match self.parent.build() {
            Ok(v) => v,
            Err(_) => {
                return Err(());
            }
        };

        Ok(self.layer.wrap(built))
    }
}

/* -------------------------------------------------------------------------- */
/*                                    Tests                                   */
/* -------------------------------------------------------------------------- */

/* -------------------------------------------------------------------------- */
/*                           Reusable Test Utilities                          */
/* -------------------------------------------------------------------------- */
