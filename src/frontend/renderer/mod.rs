use color_eyre::eyre;
use tokio::io::empty;
use url::Url;

use crate::{
    frontend::response::FrontendResponse, project::ProjectError, resolver::UrlResolutionError,
};

pub mod jinja;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct FrontendErrorInfo<'a> {
    pub status_code: Option<u16>,
    pub summary: Option<&'a str>,
    pub details: Option<&'a str>,
    pub owner: Option<&'a str>,
    pub repo: Option<&'a str>,
    pub branch: Option<&'a str>,
    pub asset: Option<&'a str>,
}

impl<'a> FrontendErrorInfo<'a> {
    pub const fn empty() -> Self {
        Self {
            status_code: None,
            summary: None,
            details: None,
            owner: None,
            repo: None,
            branch: None,
            asset: None,
        }
    }

    pub fn status(status_code: u16) -> Self {
        Self {
            status_code: Some(status_code),
            summary: match status_code {
                200 => Some("OK"),
                400 => Some("Bad Request"),
                401 => Some("Unauthorized"),
                403 => Some("Forbidden"),
                404 => Some("Not Found"),
                500 => Some("Internal Server Error"),
                501 => Some("Not Implemented"),
                502 => Some("Bad Gateway"),
                503 => Some("Service Unavailable"),
                504 => Some("Gateway Timeout"),
                _ => Some("Unknown Error"),
            },
            details: None,
            owner: None,
            repo: None,
            branch: None,
            asset: None,
        }
    }

    pub fn page_error(page_error: &ProjectError) -> Self {
        match page_error {
            ProjectError::ProviderError => Self {
                status_code: Some(400),
                summary: Some("Provider Error"),
                details: Some("An internal error occured when trying to find the page."),
                repo: None,
                owner: None,
                branch: None,
                asset: None,
            },
        }
    }

    pub fn url_resolution_error(url_error: &UrlResolutionError) -> Self {
        match url_error {
            UrlResolutionError::MalformedUrl => Self {
                status_code: Some(400),
                summary: Some("Malformed URL"),
                details: None,
                repo: None,
                owner: None,
                branch: None,
                asset: None,
            },
            UrlResolutionError::Unauthorized => Self {
                status_code: Some(401),
                summary: Some("Unauthorized"),
                details: None,
                repo: None,
                owner: None,
                branch: None,
                asset: None,
            },
        }
    }

    pub fn with_summary(mut self, summary: &'a str) -> Self {
        self.summary = Some(summary);
        self
    }

    pub fn with_details(mut self, details: &'a str) -> Self {
        self.details = Some(details);
        self
    }

    pub fn with_owner(mut self, owner: &'a str) -> Self {
        self.owner = Some(owner);
        self
    }

    pub fn with_repo(mut self, repo: &'a str) -> Self {
        self.repo = Some(repo);
        self
    }

    pub fn with_asset(mut self, asset: &'a str) -> Self {
        self.asset = Some(asset);
        self
    }

    pub fn with_branch(mut self, branch: &'a str) -> Self {
        self.branch = Some(branch);
        self
    }

    const MALFORMED_QUERY: FrontendErrorInfo<'static> = FrontendErrorInfo {
        status_code: Some(400),
        summary: Some("Malformed Query"),
        details: Some("Failed to analyze query."),
        owner: None,
        repo: None,
        branch: None,
        asset: None,
    };
}

/// A frontend renderer is responsible for handling the rendering of pages.
/// It should be divorced from the rest of the frontend logic, used for rendering an interface.
pub trait FrontendRenderer: Clone + Sync + Send {
    /// Allows rendering of custom pages, if available, based on the URL.
    /// It should take priority over what would normally be rendered for the URL.
    ///
    /// # Arguments
    ///
    /// * `url`: The URL being requested.
    ///
    /// # Returns
    ///
    /// * Some(Ok(R)): A response is rendered and available for the URL, and should be used.
    /// * Some(Err(e)): An error occurred while attempting to render a custom response.
    /// * None: No custom rendering available for this URL.
    ///
    /// # Notes
    ///
    /// * This method is optional to implement. If not implemented, it will always return None.
    fn render_custom<R: FrontendResponse>(&self, url: &Url) -> Option<R> {
        None
    }
    /// Renders an Index page to default to.
    fn render_index<R: FrontendResponse>(&self) -> R;
    /// Renders an error page based on the provided error information.
    fn render_error<R: FrontendResponse>(&self, info: &FrontendErrorInfo) -> R;
}
