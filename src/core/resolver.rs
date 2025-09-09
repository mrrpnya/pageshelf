use log::warn;
use url::Url;

use crate::{PageAssetLocation, PageLocation};

use super::util::analyze_url;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UrlResolution {
    /// The URL pointed to a page at this location.
    Page(PageAssetLocation),
    // TODO: Rethink this? Should probably override the URL resolver entirely.
    /// The URL is a built-in page.
    BuiltIn,
    /// The URL points to a domain.
    External(Url),
    /// The URL is invalid.
    Malformed(String),
}

pub trait UrlResolver {
    fn resolve(&self, url: Url) -> UrlResolution;
}

#[derive(Clone)]
pub struct DefaultUrlResolver {
    home_domain: Option<String>,
    page_domains: Option<Vec<String>>,
    external_enabled: bool,
    default_repo: String,
    default_branch: String,
}

impl DefaultUrlResolver {
    /// Creates a default URL resolver, based on the provided parameters.
    ///
    /// # Arguments
    ///
    /// - `home_domain` (`Option<Url>`) - The domain that is associated with the server directly.
    /// - `page_domains` (`Option<Vec<Url>>`) - The (wildcard) domains that also are associated with the server.
    /// - `default_repo` (`String`) - The repository to default to if none is specified.
    /// - `default_branch` (`String`) - The branch to default to if none is specified.
    /// - `external_enabled` (`bool`) - Whether or not to consider arbitrary domains.
    ///
    /// # Returns
    ///
    /// - `Self` - A URL resolver that will operate according to the above parameters.
    pub fn new(
        home_domain: Option<Url>,
        page_domains: Option<Vec<Url>>,
        default_repo: String,
        default_branch: String,
        external_enabled: bool,
    ) -> Self {
        Self {
            home_domain: match home_domain {
                Some(v) => {
                    if let Some(v) = v.host_str() {
                        Some(v.to_string())
                    } else {
                        warn!("Failed to determine home domain ({}) host", v);
                        None
                    }
                }
                None => None,
            },
            page_domains: page_domains.map(|v| {
                v.iter()
                    .map(|f| f.host_str())
                    .filter(|f| {
                        if f.is_some() {
                            return true;
                        }
                        warn!("Failed to determine page domain host");
                        false
                    })
                    .map(|f| f.unwrap().to_string())
                    .collect()
            }),
            default_repo,
            default_branch,
            external_enabled,
        }
    }
}

impl UrlResolver for DefaultUrlResolver {
    fn resolve(&self, url: Url) -> UrlResolution {
        let host = url.host_str();

        let is_root = (self.page_domains.iter().count() == 0 && !self.external_enabled)
            || match host {
                Some(host) => match &self.page_domains {
                    Some(pd) => match &self.home_domain {
                        Some(hd) => hd == host,
                        None => {
                            if pd.iter().any(|f| f == host) {
                                false
                            } else {
                                self.page_domains.iter().count() == 0 && !self.external_enabled
                            }
                        }
                    },
                    None => match &self.home_domain {
                        Some(hd) => hd == host,
                        None => !self.external_enabled,
                    },
                },
                // Automatically assume that it's the root if the host isn't specified
                None => self.page_domains.iter().count() == 0,
            };

        match is_root {
            true => match analyze_url(&url, None) {
                Some(a) => match a.owner {
                    Some(owner) => UrlResolution::Page(PageAssetLocation {
                        page: PageLocation {
                            owner,
                            name: a.repo.unwrap_or(self.default_repo.clone()),
                            branch: a.branch.unwrap_or(self.default_branch.clone()),
                        },
                        asset: a.asset,
                    }),
                    None => UrlResolution::BuiltIn,
                },
                None => UrlResolution::BuiltIn,
            },
            false => {
                let host = host.unwrap();
                match &self.page_domains {
                    Some(pds) => {
                        for pd in pds {
                            if is_in_url(pd, host) {
                                match analyze_url(&url, Some(pd)) {
                                    Some(a) => match a.owner {
                                        Some(owner) => {
                                            return UrlResolution::Page(PageAssetLocation {
                                                page: PageLocation {
                                                    owner,
                                                    name: a
                                                        .repo
                                                        .unwrap_or(self.default_repo.clone()),
                                                    branch: a
                                                        .branch
                                                        .unwrap_or(self.default_branch.clone()),
                                                },
                                                asset: a.asset,
                                            });
                                        }
                                        None => {
                                            if self.external_enabled {
                                                return UrlResolution::External(url.clone());
                                            } else {
                                                drop(UrlResolution::BuiltIn);
                                            }
                                        }
                                    },
                                    None => {
                                        continue;
                                    }
                                }
                            }
                        }
                        if self.external_enabled {
                            UrlResolution::External(url)
                        } else {
                            UrlResolution::BuiltIn
                        }
                    }
                    None => {
                        if self.external_enabled {
                            UrlResolution::External(url)
                        } else {
                            UrlResolution::BuiltIn
                        }
                    }
                }
            }
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                                URL Utilities                               */
/* -------------------------------------------------------------------------- */

fn is_in_url(url_base: &str, url: &str) -> bool {
    log::debug!("Checking if {} ends in {}...", url, url_base);
    let s = format!(".{}", url_base);
    url.ends_with(&s)
}

/* -------------------------------------------------------------------------- */
/*                                   Testing                                  */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
pub mod tests {
    use std::str::FromStr;

    use url::Url;

    use crate::{
        PageAssetLocation, PageLocation,
        resolver::{DefaultUrlResolver, UrlResolution},
    };

    use super::UrlResolver;

    #[test]
    fn root_builtin() {
        let r = DefaultUrlResolver::new(
            Some(Url::from_str("http://home.domain").unwrap()),
            Some(vec![Url::from_str("http://pages.domain").unwrap()]),
            "pages".to_string(),
            "pages".to_string(),
            false,
        );

        assert_eq!(
            r.resolve(Url::from_str("http://home.domain").unwrap()),
            UrlResolution::BuiltIn
        );

        assert_eq!(
            r.resolve(Url::from_str("http://home.domain/").unwrap()),
            UrlResolution::BuiltIn
        );

        assert_eq!(
            r.resolve(Url::from_str("http://other.domain").unwrap()),
            UrlResolution::BuiltIn
        );

        assert_eq!(
            r.resolve(Url::from_str("http://pages.domain").unwrap()),
            UrlResolution::BuiltIn
        );

        let r = DefaultUrlResolver::new(
            Some(Url::from_str("http://home.domain").unwrap()),
            Some(vec![Url::from_str("http://pages.domain").unwrap()]),
            "pages".to_string(),
            "pages".to_string(),
            true,
        );

        assert_eq!(
            r.resolve(Url::from_str("http://home.domain").unwrap()),
            UrlResolution::BuiltIn
        );

        assert_ne!(
            r.resolve(Url::from_str("http://other.domain").unwrap()),
            UrlResolution::BuiltIn
        );

        assert_eq!(
            r.resolve(Url::from_str("http://pages.domain").unwrap()),
            UrlResolution::External(Url::from_str("http://pages.domain").unwrap())
        );
    }

    /// Try and find a user via root URL
    #[test]
    fn root_user_identify() {
        let r = DefaultUrlResolver::new(
            Some(Url::from_str("http://home.domain/nya").unwrap()),
            Some(vec![Url::from_str("http://pages.domain/nya").unwrap()]),
            "pages".to_string(),
            "pages".to_string(),
            false,
        );

        assert_eq!(
            r.resolve(Url::from_str("http://home.domain/nya").unwrap()),
            UrlResolution::Page(PageAssetLocation {
                page: PageLocation {
                    owner: "nya".to_string(),
                    name: "pages".to_string(),
                    branch: "pages".to_string()
                },
                asset: "/".to_string()
            })
        );

        assert_eq!(
            r.resolve(Url::from_str("http://other.domain/nya").unwrap()),
            UrlResolution::BuiltIn
        );
    }

    #[test]
    fn default_to_root() {
        let r =
            DefaultUrlResolver::new(None, None, "pages".to_string(), "pages".to_string(), false);

        assert_eq!(
            r.resolve(Url::from_str("http://home.domain/nya").unwrap()),
            UrlResolution::Page(PageAssetLocation {
                page: PageLocation {
                    owner: "nya".to_string(),
                    name: "pages".to_string(),
                    branch: "pages".to_string()
                },
                asset: "/".to_string()
            })
        );

        assert_eq!(
            r.resolve(Url::from_str("http://other.domain/nya").unwrap()),
            UrlResolution::Page(PageAssetLocation {
                page: PageLocation {
                    owner: "nya".to_string(),
                    name: "pages".to_string(),
                    branch: "pages".to_string()
                },
                asset: "/".to_string()
            })
        );
    }

    #[test]
    fn subdomains() {
        let r = DefaultUrlResolver::new(
            None,
            Some(vec![Url::from_str("http://home.domain").unwrap()]),
            "pages".to_string(),
            "pages".to_string(),
            false,
        );

        assert_eq!(
            r.resolve(Url::from_str("http://nya.home.domain").unwrap()),
            UrlResolution::Page(PageAssetLocation {
                page: PageLocation {
                    owner: "nya".to_string(),
                    name: "pages".to_string(),
                    branch: "pages".to_string()
                },
                asset: "/".to_string()
            })
        );

        assert_eq!(
            r.resolve(Url::from_str("http://home.domain").unwrap()),
            UrlResolution::BuiltIn
        );
    }

    #[test]
    fn domains() {
        let r = DefaultUrlResolver::new(
            Some(Url::from_str("http://pages.home.domain").unwrap()),
            Some(vec![Url::from_str("http://home.domain").unwrap()]),
            "pages".to_string(),
            "pages".to_string(),
            true,
        );

        assert_eq!(
            r.resolve(Url::from_str("http://home.domain").unwrap()),
            UrlResolution::External(Url::from_str("http://home.domain").unwrap())
        );

        assert_eq!(
            r.resolve(Url::from_str("http://other.domain").unwrap()),
            UrlResolution::External(Url::from_str("http://other.domain").unwrap())
        );
    }
}
