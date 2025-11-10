use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use regex::Regex;
use url::Url;

use crate::{
    ext::Normalizable,
    resolution::{PageResolver, ResolutionError},
    upstream::{AssetLocation, PageLocation},
};

/* -------------------------------------------------------------------------- */
/*                                  Messages                                  */
/* -------------------------------------------------------------------------- */

const ERR_PATH_EMPTY: &str = "Path is empty";
const ERR_OWNER_MISSING: &str = "Owner missing";
const ERR_PROJECT_MISSING: &str = "Project missing and no default";
const ERR_CHANNEL_MISSING: &str = "Channel missing and no default";
const ERR_MALFORMED: &str = "Malformed URL";

/* -------------------------------------------------------------------------- */
/*                              Resolution result                             */
/* -------------------------------------------------------------------------- */

#[derive(Debug)]
pub struct DirectoryPageResolution {
    owner: String,
    project: String,
    channel: String,
    path: PathBuf,
}

impl PageLocation for DirectoryPageResolution {
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

impl AssetLocation for DirectoryPageResolution {
    fn path(&self) -> &Path {
        &self.path
    }
}

/* -------------------------------------------------------------------------- */
/*                                  Resolver                                  */
/* -------------------------------------------------------------------------- */

/// Resolves pages based on a virtual directory-like URL path.
///
/// # Overview
/// The `DirectoryPageResolver` interprets URL paths of this form,
/// with optional components put in parenthesis:
///
/// ```text
/// /owner/project(:channel)(/asset)
/// ```
///
/// # Behavior
///
/// - **Owner** — Always required.  
/// - **Project** — Required unless a default project is configured.
/// - **Channel** — Required unless a default channel is configured.
/// - **Asset path** — Optional, will resolve to root if omitted.
///
/// The resolver also performs normalization on the input path via these rules:
/// - Collapses multiple consecutive slashes (`//`) into a single `/`.
/// - Normalizes relative segments (`/./` and `/../`).
/// - Removes any leading slash in the final asset path (though trailing slashes are preserved).
///
/// # Examples
///
/// ## Usage
///
/// ```ignore
/// // default() has defaults included
/// let resolver = DirectoryPageResolver::default();
///
/// resolver.resolve("/johndoe"); // The default channel of the default project
/// resolver.resolve("/johndoe/personalsite:nightly/my/asset"); // The "nightly" channel of the "personalsite" project
/// ```
///
/// ## Valid paths
///
/// ```text
/// /johndoe/personalsite
/// /johndoe/personalsite:nightly
/// /johndoe//personalsite
/// /johndoe//personalsite:nightly
/// /johndoe/personalsite/my/asset
/// /johndoe/personalsite:nightly/my/asset
/// /johndoe
/// /johndoe:nightly
/// ```
///
/// # Notes
///
/// - Projects and channels will not be inferred if defaults are not provided.
// TODO: Make the example above a doctest
pub struct DirectoryPageResolver {
    default_project: Option<String>,
    default_channel: Option<String>,
}

impl DirectoryPageResolver {
    /// Creates a new [DirectoryPageResolver].
    ///
    /// # Arguments
    ///
    /// - `default_project` - Default to this project if the URL doesn't specify.
    ///   Recommended to set to "pages" as a reasonable default.
    /// - `default_channel` - Default to this channel if the URL doesn't specify.
    ///   Recommended to set to "pages" as a reasonable default.
    ///
    /// If these are not set, specifying them explicitly will be mandatory when querying.
    /// If you are going to set both to "pages", it's better to use `DirectoryPageResolver::default()`.
    pub fn new(default_project: Option<String>, default_channel: Option<String>) -> Self {
        Self {
            default_project,
            default_channel,
        }
    }
}

impl Default for DirectoryPageResolver {
    fn default() -> Self {
        Self::new(Some("pages".to_string()), Some("pages".to_string()))
    }
}

// Solely focuses on the path:
// /owner/project(:channel)(/asset)
// - If project is missing, will infer default project (or fail)
// - If channel is missing, will infer default channel (or fail)
// - Asset will be inferred to be root if missing or if simply "/".
// - Asset paths are normalized such that /../ or /./ is processed, and there is no leading slash.
//   - There can be a trailing slash however (if it is a directory).
// Examples of valid paths:
// /johndoe/personalsite
// /johndoe/personalsite:nightly
// /johndoe//personalsite
// /johndoe//personalsite:nightly
// /johndoe/personalsite/my/asset
// /johndoe/personalsite:nightly/my/asset
// /johndoe
// /johndoe:nightly
// Multiple slashes are allowed, and are converted to one slash. Leading slashes are also allowed.
// Steps:
// - Perform validation against a schema
// - Extract owner, project, channel, and asset into Options
// - Validate said options
impl PageResolver for DirectoryPageResolver {
    // TODO: Change this to use proper Regex schemas
    async fn resolve_url(&self, url: &Url) -> Result<Arc<dyn AssetLocation>, ResolutionError> {
        // Special-case empty or single slash: these indicate owner missing per tests
        let original_path = url.path();
        if original_path.is_empty() || original_path == "/" {
            return Err(ResolutionError::Invalid(ERR_OWNER_MISSING));
        }

        // Normalize path: collapse multiple slashes into one
        let re = Regex::new(r"/+").expect("The regex meant to help collapse slashes was invalid");
        let normalized = re.replace_all(original_path, "/").to_string();

        // If normalization produces a single slash (original was multiple slashes) treat as empty path
        if normalized == "/" {
            return Err(ResolutionError::Invalid(ERR_PATH_EMPTY));
        }

        // Trim leading slash for easier splitting
        let trimmed = normalized.trim_start_matches('/');

        let segments: Vec<&str> = trimmed.split('/').filter(|s| !s.is_empty()).collect();

        // Owner is always first segment
        let owner_seg = segments.first().copied().unwrap_or("");
        if owner_seg.is_empty() {
            return Err(ResolutionError::Invalid(ERR_OWNER_MISSING));
        }

        // Helper for splitting a single colon into two parts, returning owned Strings
        let split_colon = |s: &str| -> Option<(String, String)> {
            let mut parts = s.splitn(2, ':');
            let a = parts.next()?;
            let b = parts.next().unwrap_or("");
            // If there's an extra colon it will be part of b; detect multiple colons by checking if b contains ':'
            if b.contains(':') {
                return None;
            }
            Some((a.to_string(), b.to_string()))
        };

        // parse
        if owner_seg.contains(':') {
            // owner:channel form
            let (o, c) = match split_colon(owner_seg) {
                Some((o, c)) if !o.is_empty() && !c.is_empty() => (o, c),
                _ => return Err(ResolutionError::Invalid(ERR_MALFORMED)),
            };

            let owner = o.to_string();

            // project must be defaulted
            let project = match &self.default_project {
                Some(p) => p.clone(),
                None => return Err(ResolutionError::Invalid(ERR_PROJECT_MISSING)),
            };

            // channel from owner segment
            let channel = c.to_string();

            // asset is rest of segments after owner; if none -> root '/'
            let asset = if segments.len() <= 1 {
                PathBuf::from("/")
            } else {
                let rest = &segments[1..];
                let joined = rest.join("/");
                PathBuf::from(joined.to_string())
            };

            return Ok(Arc::new(DirectoryPageResolution {
                owner,
                project,
                channel,
                path: asset,
            }));
        }

        // owner without channel
        let owner = owner_seg.to_string();

        if segments.len() == 1 {
            // No project segment
            let project = match &self.default_project {
                Some(p) => p.clone(),
                None => return Err(ResolutionError::Invalid(ERR_PROJECT_MISSING)),
            };
            let channel = match &self.default_channel {
                Some(c) => c.clone(),
                None => return Err(ResolutionError::Invalid(ERR_CHANNEL_MISSING)),
            };
            return Ok(Arc::new(DirectoryPageResolution {
                owner,
                project,
                channel,
                path: PathBuf::from("/"),
            }));
        }

        // There is a project segment
        let proj_seg = segments[1];
        if proj_seg.is_empty() {
            return Err(ResolutionError::Invalid(ERR_MALFORMED));
        }

        // project may include channel
        if proj_seg.contains(':') {
            let (p, c) = match split_colon(proj_seg) {
                Some((p, c)) if !p.is_empty() && !c.is_empty() => (p, c),
                _ => return Err(ResolutionError::Invalid(ERR_MALFORMED)),
            };

            let project = p.to_string();
            let channel = c.to_string();

            // asset is remainder after project
            let path = if segments.len() <= 2 {
                PathBuf::from("/")
            } else {
                let rest = &segments[2..];
                PathBuf::from(rest.join("/")).normalized_relative()
            };

            return Ok(Arc::new(DirectoryPageResolution {
                owner,
                project,
                channel,
                path,
            }));
        }

        // project explicit without channel
        let project = proj_seg.to_string();
        let channel = match &self.default_channel {
            Some(c) => c.clone(),
            None => return Err(ResolutionError::Invalid(ERR_CHANNEL_MISSING)),
        };

        let path = if segments.len() <= 2 {
            PathBuf::from("/")
        } else {
            let rest = &segments[2..];
            PathBuf::from(rest.join("/")).normalized_relative()
        };

        Ok(Arc::new(DirectoryPageResolution {
            owner,
            project,
            channel,
            path,
        }))
    }
}

#[cfg(test)]
mod tests {
    use rstest::*;
    use url::Url;

    use crate::resolution::{
        PageResolver, ResolutionError,
        directory::{
            DirectoryPageResolver, ERR_CHANNEL_MISSING, ERR_MALFORMED, ERR_OWNER_MISSING,
            ERR_PATH_EMPTY, ERR_PROJECT_MISSING,
        },
    };

    /* ---------------------------------- Valid --------------------------------- */

    #[rstest]
    #[case("/johndoe", "johndoe", "defaultproj", "defaultchannel", "/")]
    #[case("/johndoe/", "johndoe", "defaultproj", "defaultchannel", "/")]
    #[case("/johndoe:nightly", "johndoe", "defaultproj", "nightly", "/")]
    #[case("/johndoe:nightly/", "johndoe", "defaultproj", "nightly", "/")]
    #[case(
        "/johndoe/personalsite",
        "johndoe",
        "personalsite",
        "defaultchannel",
        "/"
    )]
    #[case(
        "/johndoe/personalsite/",
        "johndoe",
        "personalsite",
        "defaultchannel",
        "/"
    )]
    #[case(
        "/johndoe/personalsite:nightly/",
        "johndoe",
        "personalsite",
        "nightly",
        "/"
    )]
    #[case(
        "/johndoe/personalsite/my/asset",
        "johndoe",
        "personalsite",
        "defaultchannel",
        "my/asset"
    )]
    #[case(
        "/johndoe/personalsite/my/asset/",
        "johndoe",
        "personalsite",
        "defaultchannel",
        "my/asset"
    )]
    #[case(
        "/johndoe/personalsite:nightly/my/asset",
        "johndoe",
        "personalsite",
        "nightly",
        "my/asset"
    )]
    #[case(
        "/johndoe/personalsite:nightly/my/asset/",
        "johndoe",
        "personalsite",
        "nightly",
        "my/asset"
    )]
    #[tokio::test]
    async fn defaults_valid_directories(
        #[case] url: &str,
        #[case] owner: &str,
        #[case] project: &str,
        #[case] channel: &str,
        #[case] path: &str,
    ) {
        let resolver = DirectoryPageResolver::new(
            Some("defaultproj".to_string()),
            Some("defaultchannel".to_string()),
        );
        let url = Url::parse(&format!("https://example.com{}", url)).unwrap();
        let res = resolver.resolve_url(&url).await;
        assert!(res.is_ok());

        let res = res.unwrap();

        assert_eq!(res.owner(), owner);
        assert_eq!(res.project(), project);
        assert_eq!(res.channel(), channel);
        assert_eq!(res.path(), std::path::Path::new(path));
    }

    #[rstest]
    #[case("/johndoe/site:channel", "johndoe", "site", "channel", "/")]
    #[case("/johndoe//site:channel/", "johndoe", "site", "channel", "/")]
    #[case("/johndoe///site:channel", "johndoe", "site", "channel", "/")]
    #[case("//johndoe//site:channel", "johndoe", "site", "channel", "/")]
    #[case("///johndoe//site:channel", "johndoe", "site", "channel", "/")]
    #[tokio::test]
    async fn no_defaults_valid_directories(
        #[case] url: &str,
        #[case] owner: &str,
        #[case] project: &str,
        #[case] channel: &str,
        #[case] path: &str,
    ) {
        let resolver = DirectoryPageResolver::new(None, None);
        let url = Url::parse(&format!("https://example.com{}", url)).unwrap();
        let res = resolver.resolve_url(&url).await;
        assert!(res.is_ok());

        let res = res.unwrap();

        assert_eq!(res.owner(), owner);
        assert_eq!(res.project(), project);
        assert_eq!(res.channel(), channel);
        assert_eq!(res.path(), std::path::Path::new(path));
    }

    #[rstest]
    #[case("/johndoe:channel", "johndoe", "defaultproj", "channel", "/")]
    #[case("/johndoe:channel/", "johndoe", "defaultproj", "channel", "/")]
    #[case("/johndoe:channel/asset", "johndoe", "defaultproj", "channel", "asset")]
    #[case("/johndoe/site:channel/", "johndoe", "site", "channel", "/")]
    #[case("/johndoe//site:channel", "johndoe", "site", "channel", "/")]
    #[case("//johndoe:channel", "johndoe", "defaultproj", "channel", "/")]
    #[case("///johndoe:channel", "johndoe", "defaultproj", "channel", "/")]
    #[case(
        "///johndoe:channel/asset",
        "johndoe",
        "defaultproj",
        "channel",
        "asset"
    )]
    #[tokio::test]
    async fn project_defaults_valid_directories(
        #[case] url: &str,
        #[case] owner: &str,
        #[case] project: &str,
        #[case] channel: &str,
        #[case] path: &str,
    ) {
        let resolver = DirectoryPageResolver::new(Some("defaultproj".to_string()), None);
        let url = Url::parse(&format!("https://example.com{}", url)).unwrap();
        let res = resolver.resolve_url(&url).await;
        assert!(res.is_ok());

        let res = res.unwrap();

        assert_eq!(res.owner(), owner);
        assert_eq!(res.project(), project);
        assert_eq!(res.channel(), channel);
        assert_eq!(res.path(), std::path::Path::new(path));
    }

    /* --------------------------------- Invalid -------------------------------- */

    // Without channel/project
    #[rstest]
    #[case("", ERR_OWNER_MISSING)]
    #[case("/", ERR_OWNER_MISSING)]
    #[case("/:", ERR_MALFORMED)]
    #[case("/owner", ERR_PROJECT_MISSING)]
    #[case("/owner:", ERR_MALFORMED)]
    #[case("/owner::", ERR_MALFORMED)]
    #[case("/owner/:", ERR_MALFORMED)]
    #[case("/owner//asset", ERR_CHANNEL_MISSING)]
    #[case("/owner::asset", ERR_MALFORMED)]
    #[case("///", ERR_PATH_EMPTY)]
    #[case("/owner:channel:extra", ERR_MALFORMED)]
    #[tokio::test]
    async fn no_defaults_invalid_directories(#[case] url: &str, #[case] expected_error: &str) {
        let resolver = DirectoryPageResolver::new(None, None);
        let url = Url::parse(&format!("https://example.com{}", url)).unwrap();
        let res = resolver.resolve_url(&url).await;
        assert!(res.is_err());
        match res {
            Err(ResolutionError::Invalid(msg)) => {
                assert_eq!(msg, expected_error);
            }
            _ => panic!("Expected InvalidUrl error"),
        }
    }
}
