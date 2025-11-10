use std::{fmt::Debug, path::Path};

/// Contains a full location to a page
pub trait PageLocation: Debug + Send + Sync {
    /// Returns the owner of the page.
    ///
    /// This is equivalent to Git user names.
    fn owner(&self) -> &str;
    /// Returns the project the page is on.
    ///
    /// This is equivalent to Git repo names.
    fn project(&self) -> &str;
    /// Returns the channel the page is on.
    ///
    /// This is equivalent to Git branch names.
    fn channel(&self) -> &str;
}

/// Contains a full location to a given asset within a page
pub trait AssetLocation: PageLocation {
    /// Returns the path of an asset
    fn path(&self) -> &Path;
}
