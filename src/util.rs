use actix_web::body::None;
use url::Url;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UrlAnalysis {
    pub owner: Option<String>,
    pub repo: Option<String>,
    pub branch: Option<String>,
    pub asset: String,
}

pub fn analyze_url(url: &Url, pages_url: &Url) -> Option<UrlAnalysis> {
    let host = url.host_str()?;
    let base_host = pages_url.host_str()?;

    // If host is unrelated
    if host != base_host && !host.ends_with(&format!(".{base_host}")) {
        return None;
    }

    let mut owner = None;
    let mut repo = None;
    let mut branch = None;
    let mut asset = "/".to_string();

    if host != base_host {
        // --- Subdomain-based form ---
        // Example: unstable.page.person.example.domain
        let prefix = host
            .strip_suffix(base_host)?
            .strip_suffix('.')
            .unwrap_or(host);
        let mut parts = prefix.rsplitn(3, '.'); // reverse split, up to 3 parts

        if let Some(o) = parts.next() {
            owner = Some(o.to_string());
        }
        if let Some(r) = parts.next() {
            repo = Some(r.to_string());
        }
        if let Some(b) = parts.next() {
            branch = Some(b.to_string());
        }

        // Collect path segments into asset if present
        let path: String = url
            .path_segments()?
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("/");
        if !path.is_empty() {
            asset.push_str(&path);
        }
    } else {
        // --- Path-based form ---
        let mut segments = url.path_segments()?.filter(|s| !s.is_empty());

        if let Some(o) = segments.next() {
            owner = Some(o.to_string());
        }
        if let Some(r) = segments.next() {
            if let Some((repo_part, branch_part)) = r.split_once(':') {
                repo = Some(repo_part.to_string());
                branch = Some(branch_part.to_string());
            } else {
                repo = Some(r.to_string());
            }
        }
        let path: String = segments.collect::<Vec<_>>().join("/");
        if !path.is_empty() {
            asset.push_str(&path);
        }
    }

    Some(UrlAnalysis {
        owner,
        repo,
        branch,
        asset,
    })
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use url::Url;

    use super::{UrlAnalysis, analyze_url};

    #[test]
    fn test_analyze_url_subdirectory() {
        let domain = Url::from_str("http://example.domain").unwrap();

        let params: Vec<(&str, Option<UrlAnalysis>)> = vec![
            ("other.domain", None),
            ("other.domain/my_asset", None),
            (
                "example.domain",
                Some(UrlAnalysis {
                    owner: None,
                    repo: None,
                    branch: None,
                    asset: "/".to_string(),
                }),
            ),
            (
                "example.domain/person",
                Some(UrlAnalysis {
                    owner: Some("person".to_string()),
                    repo: None,
                    branch: None,
                    asset: "/".to_string(),
                }),
            ),
            (
                "example.domain/person/page",
                Some(UrlAnalysis {
                    owner: Some("person".to_string()),
                    repo: Some("page".to_string()),
                    branch: None,
                    asset: "/".to_string(),
                }),
            ),
            (
                "example.domain/person/page:unstable",
                Some(UrlAnalysis {
                    owner: Some("person".to_string()),
                    repo: Some("page".to_string()),
                    branch: Some("unstable".to_string()),
                    asset: "/".to_string(),
                }),
            ),
        ];

        for param in params {
            let url_str = format!("http://{}", param.0);
            let url = Url::from_str(url_str.as_str()).unwrap();
            let a = analyze_url(&url, &domain);
            assert_eq!(a, param.1, "Analyzing {}", param.0)
        }
    }

    #[test]
    fn test_analyze_url_subdomain() {
        let domain = Url::from_str("http://example.domain").unwrap();

        let params: Vec<(&str, Option<UrlAnalysis>)> = vec![
            ("person.other.domain", None),
            ("person.other.domain/my_asset", None),
            ("page.person.other.domain", None),
            ("page.person.other.domain/my_asset", None),
            ("unstable.page.person.other.domain", None),
            ("unstable.page.person.other.domain/my_asset", None),
            (
                "person.example.domain",
                Some(UrlAnalysis {
                    owner: Some("person".to_string()),
                    repo: None,
                    branch: None,
                    asset: "/".to_string(),
                }),
            ),
            (
                "person.example.domain/my_asset",
                Some(UrlAnalysis {
                    owner: Some("person".to_string()),
                    repo: None,
                    branch: None,
                    asset: "/my_asset".to_string(),
                }),
            ),
            (
                "page.person.example.domain/my_asset",
                Some(UrlAnalysis {
                    owner: Some("person".to_string()),
                    repo: Some("page".to_string()),
                    branch: None,
                    asset: "/my_asset".to_string(),
                }),
            ),
            (
                "unstable.page.person.example.domain",
                Some(UrlAnalysis {
                    owner: Some("person".to_string()),
                    repo: Some("page".to_string()),
                    branch: Some("unstable".to_string()),
                    asset: "/".to_string(),
                }),
            ),
            (
                "unstable.page.person.example.domain/my_asset",
                Some(UrlAnalysis {
                    owner: Some("person".to_string()),
                    repo: Some("page".to_string()),
                    branch: Some("unstable".to_string()),
                    asset: "/my_asset".to_string(),
                }),
            ),
        ];

        for param in params {
            let url_str = format!("http://{}", param.0);
            let url = Url::from_str(url_str.as_str()).unwrap();
            let a = analyze_url(&url, &domain);
            assert_eq!(a, param.1, "Analyzing {}", param.0)
        }
    }
}
