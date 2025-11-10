use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

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
//const ERR_OWNER_MISSING: &str = "Owner missing";
const ERR_PROJECT_MISSING: &str = "Project missing and no default";
const ERR_CHANNEL_MISSING: &str = "Channel missing and no default";
const ERR_MALFORMED: &str = "Malformed URL";

/* -------------------------------------------------------------------------- */
/*                              Resolution result                             */
/* -------------------------------------------------------------------------- */

#[derive(Debug)]
pub struct SubdomainAssetLocation {
    owner: String,
    project: String,
    channel: String,
    path: PathBuf,
}

impl PageLocation for SubdomainAssetLocation {
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

impl AssetLocation for SubdomainAssetLocation {
    fn path(&self) -> &Path {
        &self.path
    }
}

/// Resolves pages based on subdomains of a configured domain list.
///
/// # Overview
/// The `SubdomainPageResolver` interprets subdomain-based URLs of this form,
/// with optional components shown in parentheses:
///
/// ```text
/// ((channel).project).owner.[allowed-domain]
/// ```
///
/// # Behavior
///
/// - **Owner** — Always required.  
/// - **Project** — Required unless a default project is configured.  
/// - **Channel** — Required unless a default channel is configured.
/// - **Allowed domains** — The domain suffix must match one of the configured `allowed_domains` entries.  
/// - **Asset path** — Optional, will resolve to root if omitted.
///
/// # Examples
///
/// ## Usage
///
/// ```ignore
/// let resolver = SubdomainPageResolver::new(
///     Some(vec!["example.com".into()]),
///     Some("mainproject".into()),
///     Some("stable".into()),
/// );
///
/// // Resolves to johndoe’s default project/channel on example.com
/// resolver.resolve_url("https://johndoe.example.com");
///
/// // Resolves to the "nightly" channel of johndoe’s "personalsite" project
/// resolver.resolve_url("https://nightly.personalsite.johndoe.example.com");
/// ```
///
/// ## Valid subdomains
///
/// ```text
/// johndoe.example.com
/// personalsite.johndoe.example.com
/// nightly.personalsite.johndoe.example.com
/// ```
pub struct SubdomainPageResolver {
    default_project: Option<String>,
    default_channel: Option<String>,
    allowed_domains: Option<Vec<String>>,
}

impl SubdomainPageResolver {
    /// Creates a new [SubdomainPageResolver].
    ///
    /// # Arguments
    ///
    /// - `allowed_domains` - A set of domains of which subdomains are allowed to be resolved.
    ///   If not set, any domain with at least three parts can be resolved.
    /// - `default_project` - Default to this project if the URL doesn't specify.
    ///   Recommended to set to "pages" as a reasonable default.
    /// - `default_channel` - Default to this channel if the URL doesn't specify.
    ///   Recommended to set to "pages" as a reasonable default.
    ///
    /// If the last two are not set, specifying them explicitly will be mandatory when querying.
    pub fn new(
        allowed_domains: Option<Vec<String>>,
        default_project: Option<String>,
        default_channel: Option<String>,
    ) -> Self {
        Self {
            default_project,
            default_channel,
            allowed_domains,
        }
    }
}

impl PageResolver for SubdomainPageResolver {
    // Solely focuseses on the subdomain:
    // ((channel).project).owner.[any of the self.allowed_domains]
    // - If project is missing, will infer default project (or fail)
    // - If channel is missing, will infer default channel (or fail)
    // Examples of valid subdomain:
    // johndoe.example.com
    // personalsite.johndoe.example.com
    // nightly.personalsite.johndoe.example.com
    // Steps:
    // - Perform validation against a regex schema
    // - Extract owner, project, channel, and asset (which is the path, use Path.normalize_relative() first tho) into Options
    // - Validate said options
    // Notes:
    // - Valid domains always have three or more elements separated by a dot.
    async fn resolve_url(&self, url: &Url) -> Result<Arc<dyn AssetLocation>, ResolutionError> {
        let host = url
            .host_str()
            .ok_or(ResolutionError::Invalid(ERR_MALFORMED))?;

        let parts: Vec<&str> = host.split('.').collect();
        if parts.len() < 3 {
            return Err(ResolutionError::Invalid(ERR_MALFORMED));
        }

        // Check allowed domain
        let allowed = if let Some(allowed_domains) = &self.allowed_domains {
            allowed_domains.iter().any(|d| host.ends_with(d))
        } else {
            true
        };
        if !allowed {
            return Err(ResolutionError::Invalid(ERR_MALFORMED));
        }

        // Find which allowed domain suffix matches, and remove it + preceding dot parts
        let (subdomain_parts, _domain_match) = if let Some(allowed_domains) = &self.allowed_domains
        {
            let mut found = None;
            for domain in allowed_domains {
                if host.ends_with(domain) {
                    let count = domain.split('.').count();
                    // remove domain and one dot part before domain
                    let sub_len = parts.len() - count;
                    found = Some(parts[..sub_len].to_vec());
                    break;
                }
            }
            (found.unwrap_or_default(), true)
        } else {
            (parts.clone(), true)
        };

        if subdomain_parts.is_empty() || subdomain_parts.iter().any(|s| s.is_empty()) {
            return Err(ResolutionError::Invalid(ERR_MALFORMED));
        }

        // Determine owner, project, channel
        let (owner, project, channel) = match subdomain_parts.len() {
            1 => {
                let owner = subdomain_parts[0];
                let project = self
                    .default_project
                    .as_deref()
                    .ok_or(ResolutionError::Invalid(ERR_PROJECT_MISSING))?;
                let channel = self
                    .default_channel
                    .as_deref()
                    .ok_or(ResolutionError::Invalid(ERR_CHANNEL_MISSING))?;
                (owner.to_string(), project.to_string(), channel.to_string())
            }
            2 => {
                let project = subdomain_parts[0];
                let owner = subdomain_parts[1];
                let channel = self
                    .default_channel
                    .as_deref()
                    .ok_or(ResolutionError::Invalid(ERR_CHANNEL_MISSING))?;
                (owner.to_string(), project.to_string(), channel.to_string())
            }
            3 => {
                let channel = subdomain_parts[0];
                let project = subdomain_parts[1];
                let owner = subdomain_parts[2];
                (owner.to_string(), project.to_string(), channel.to_string())
            }
            _ => return Err(ResolutionError::Invalid(ERR_MALFORMED)),
        };

        // Parse path
        let path_str = url.path().trim_start_matches('/');
        let path = if path_str.is_empty() {
            PathBuf::from("/")
        } else {
            PathBuf::from(path_str).normalized_relative()
        };

        if path.as_os_str().is_empty() {
            return Err(ResolutionError::Invalid(ERR_PATH_EMPTY));
        }

        Ok(Arc::new(SubdomainAssetLocation {
            owner,
            project,
            channel,
            path,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use url::Url;

    // Helper: create provider with defaults
    fn make_provider() -> SubdomainPageResolver {
        SubdomainPageResolver::new(
            Some(vec!["example.com".into()]),
            Some("defaultproj".into()),
            Some("stable".into()),
        )
    }

    #[rstest]
    #[case("https://johndoe.example.com", "johndoe", "defaultproj", "stable", "/")]
    #[case(
        "https://personalsite.johndoe.example.com",
        "johndoe",
        "personalsite",
        "stable",
        "/"
    )]
    #[case(
        "https://nightly.personalsite.johndoe.example.com",
        "johndoe",
        "personalsite",
        "nightly",
        "/"
    )]
    #[tokio::test]
    async fn resolves_valid_subdomains(
        #[case] url: &str,
        #[case] expected_owner: &str,
        #[case] expected_project: &str,
        #[case] expected_channel: &str,
        #[case] expected_path: &str,
    ) {
        let provider = make_provider();
        let url = Url::parse(url).unwrap();
        let result = provider.resolve_url(&url).await;
        assert!(result.is_ok(), "Expected success: {:?}", result);

        let loc = result.unwrap();
        assert_eq!(loc.owner(), expected_owner);
        assert_eq!(loc.project(), expected_project);
        assert_eq!(loc.channel(), expected_channel);
        assert_eq!(loc.path(), Path::new(expected_path));
    }

    #[rstest]
    #[case("https://example.com")] // No owner
    #[case("https://unknown.other.com")] // Disallowed domain
    #[case("https://..example.com")] // Malformed
    #[tokio::test]
    async fn rejects_invalid_urls(#[case] url: &str) {
        let provider = make_provider();
        let url = Url::parse(url).unwrap();
        let result = provider.resolve_url(&url).await;
        assert!(result.is_err(), "Expected error, got {:?}", result);
    }

    #[rstest]
    #[tokio::test]
    async fn fails_without_defaults() {
        let provider = SubdomainPageResolver::new(
            Some(vec!["example.com".into()]),
            None, // no default project
            None, // no default channel
        );

        let url = Url::parse("https://johndoe.example.com").unwrap();
        let result = provider.resolve_url(&url).await;
        assert!(result.is_err(), "Expected project/channel missing error");
    }

    #[rstest]
    #[tokio::test]
    async fn path_parsing_works() {
        let provider = make_provider();
        let url =
            Url::parse("https://nightly.personalsite.johndoe.example.com/assets/img/logo.png")
                .unwrap();
        let result = provider.resolve_url(&url).await.unwrap();

        assert_eq!(result.path(), Path::new("assets/img/logo.png"));
    }
}
