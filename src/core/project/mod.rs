mod owner;
pub mod source;

pub use owner::*;
pub mod layer;
use crate::core::asset::Asset;
use crate::{AssetError, AssetSource};
use std::{fmt::Display, path::Path};

/* -------------------------------- Constants ------------------------------- */

// TODO: Allow changing behavior regarding handing of domain files
/// The relative location in which to find page domain configuration within a branch.
pub const DOMAIN_FILE_PATH: &str = "/.domain";

/* -------------------------------- Utilities ------------------------------- */

#[derive(Debug, PartialEq, Eq)]
pub enum ProjectError {
    /// Something went wrong in the Page Provider.
    ProviderError,
}

impl Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProviderError => f.write_str("Provider error"),
        }
    }
}

impl std::error::Error for ProjectError {}

/* -------------------------------------------------------------------------- */
/*                               Page Accessing                               */
/* -------------------------------------------------------------------------- */

pub trait Project: Send {
    type Page<'a>: Page + 'a
    where
        Self: 'a;
    type Error: std::error::Error;

    fn name(&self) -> &str;
    async fn channels<'a>(
        &'a self,
    ) -> Result<impl Iterator<Item = Self::Page<'a>> + 'a, Self::Error>;
    fn default_channel(&self) -> Option<&str>;
    async fn get_channel<'a>(&'a self, name: &str) -> Result<Option<Self::Page<'a>>, Self::Error> {
        self.channels()
            .await
            .map(|i| i.filter(|f| f.name() == name).next())
    }
}
/// A Page represents a specific site to be hosted.
pub trait Page: AssetSource + Send {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    async fn domains(&self) -> Result<impl Iterator<Item = String>, AssetError> {
        let asset = self.get_asset(Path::new("/.domain")).await?;
        let bytes = asset.bytes();
        let body = std::str::from_utf8(bytes).map_err(|_| AssetError::CannotInterpret)?;
        let trimmed_body_lines: Vec<String> =
            body.lines().map(|line| line.trim().to_string()).collect();

        Ok(trimmed_body_lines.into_iter())
    }
}
