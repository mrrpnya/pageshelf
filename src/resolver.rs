use actix_web::HttpRequest;
use log::warn;
use url::Url;

use crate::{
    page::{PageAssetLocation, PageLocation},
    util::analyze_url,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UrlResolution {
    Page(PageAssetLocation),
    BuiltIn,
    External(Url),
    Malformed(String),
}

pub struct UrlResolver {
    home_domain: Option<String>,
    page_domains: Option<Vec<String>>,
    external_enabled: bool,
    default_repo: String,
    default_branch: String,
}

impl UrlResolver {
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
            page_domains: match page_domains {
                Some(v) => Some(
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
                        .collect(),
                ),
                None => None,
            },
            default_repo,
            default_branch,
            external_enabled,
        }
    }

    pub fn resolve(&self, url: Url) -> UrlResolution {
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
                    Some(owner) => {
                        return UrlResolution::Page(PageAssetLocation {
                            page: PageLocation {
                                owner: owner,
                                name: a.repo.unwrap_or(self.default_repo.clone()),
                                branch: a.branch.unwrap_or(self.default_branch.clone()),
                            },
                            asset: a.asset,
                        });
                    }
                    None => {
                        return UrlResolution::BuiltIn;
                    }
                },
                None => UrlResolution::BuiltIn,
            },
            false => {
                let host = host.unwrap();
                match &self.page_domains {
                    Some(pds) => {
                        for pd in pds {
                            if is_in_url(&pd, host) {
                                match analyze_url(&url, Some(&pd)) {
                                    Some(a) => match a.owner {
                                        Some(owner) => {
                                            return UrlResolution::Page(PageAssetLocation {
                                                page: PageLocation {
                                                    owner: owner,
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
                                                UrlResolution::External(url.clone());
                                            } else {
                                                UrlResolution::BuiltIn;
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

    pub fn resolve_http_request(&self, req: &HttpRequest) -> UrlResolution {
        self.resolve(req.full_url())
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
        page::{PageAssetLocation, PageLocation},
        resolver::UrlResolution,
    };

    use super::UrlResolver;

    #[test]
    fn root_builtin() {
        let r = UrlResolver::new(
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

        let r = UrlResolver::new(
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

    #[test]
    fn root_user_identify() {
        let r = UrlResolver::new(
            Some(Url::from_str("http://home.domain/mrrp").unwrap()),
            Some(vec![Url::from_str("http://pages.domain/mrrp").unwrap()]),
            "pages".to_string(),
            "pages".to_string(),
            false,
        );

        assert_eq!(
            r.resolve(Url::from_str("http://home.domain/mrrp").unwrap()),
            UrlResolution::Page(PageAssetLocation {
                page: PageLocation {
                    owner: "mrrp".to_string(),
                    name: "pages".to_string(),
                    branch: "pages".to_string()
                },
                asset: "/".to_string()
            })
        );

        assert_eq!(
            r.resolve(Url::from_str("http://other.domain/mrrp").unwrap()),
            UrlResolution::BuiltIn
        );
    }

    #[test]
    fn default_to_root() {
        let r = UrlResolver::new(
            None,
            None,
            "pages".to_string(),
            "pages".to_string(),
            false,
        );

        assert_eq!(
            r.resolve(Url::from_str("http://home.domain/mrrp").unwrap()),
            UrlResolution::Page(PageAssetLocation {
                page: PageLocation {
                    owner: "mrrp".to_string(),
                    name: "pages".to_string(),
                    branch: "pages".to_string()
                },
                asset: "/".to_string()
            })
        );

        assert_eq!(
            r.resolve(Url::from_str("http://other.domain/mrrp").unwrap()),
            UrlResolution::Page(PageAssetLocation {
                page: PageLocation {
                    owner: "mrrp".to_string(),
                    name: "pages".to_string(),
                    branch: "pages".to_string()
                },
                asset: "/".to_string()
            })
        );
    }

    #[test]
    fn subdomains() {
        let r = UrlResolver::new(
            None,
            Some(vec![Url::from_str("http://home.domain").unwrap()]),
            "pages".to_string(),
            "pages".to_string(),
            false,
        );

        assert_eq!(
            r.resolve(Url::from_str("http://mrrp.home.domain").unwrap()),
            UrlResolution::Page(PageAssetLocation {
                page: PageLocation {
                    owner: "mrrp".to_string(),
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
        let r = UrlResolver::new(
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
