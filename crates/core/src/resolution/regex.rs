use config::Config;
use regex::Regex;

use crate::resolution::PageFilter;

/* -------------------------------------------------------------------------- */
/*                                   Helpers                                  */
/* -------------------------------------------------------------------------- */

struct RegexRequirements {
    // If empty, any (that isn't blacklisted), otherwise only those that match this
    pub allow: Vec<Regex>,
    // Checked first
    pub deny: Vec<Regex>,
}

impl RegexRequirements {
    pub fn check(&self, s: &str) -> bool {
        if self.deny.iter().any(|re| re.is_match(s)) {
            return false;
        }

        if self.allow.is_empty() {
            return true;
        }

        self.allow.iter().any(|re| re.is_match(s))
    }
}

/* -------------------------------------------------------------------------- */
/*                                   Filter                                   */
/* -------------------------------------------------------------------------- */

pub struct RegexPageFilterRules {
    url: RegexRequirements,
    owner: RegexRequirements,
    project: RegexRequirements,
    channel: RegexRequirements,
    asset: RegexRequirements,
}

impl RegexPageFilterRules {
    /// # Examples
    ///
    /// ```toml
    /// [security.filter] # All fields optional
    /// url = "foo" # Allow (implicit)
    /// asset = { allow = ""} # Explicit allow/deny
    /// channel = {deny = ""}
    /// project = {allow = "", deny = ""}
    /// owner = ["", ""] # Any of them can be an array
    /// ```
    pub fn from_config(config: &Config) -> Option<RegexPageFilterRules> {
        let root = config.get_table("security.filter").ok()?;

        fn regexes(value: &config::Value) -> Vec<Regex> {
            match value.kind {
                config::ValueKind::String(ref s) if !s.is_empty() => {
                    Regex::new(s).ok().into_iter().collect()
                }
                config::ValueKind::Array(ref arr) => arr
                    .iter()
                    .filter_map(|v| v.clone().into_string().ok())
                    .filter_map(|s| Regex::new(&s).ok())
                    .collect(),
                _ => vec![],
            }
        }

        fn requirements(value: &config::Value) -> RegexRequirements {
            match value.kind {
                config::ValueKind::Table(ref t) => RegexRequirements {
                    allow: t.get("allow").map(regexes).unwrap_or_default(),
                    deny: t.get("deny").map(regexes).unwrap_or_default(),
                },
                _ => RegexRequirements {
                    allow: regexes(value),
                    deny: vec![],
                },
            }
        }

        let get = |key| {
            root.get(key)
                .map(requirements)
                .unwrap_or_else(|| RegexRequirements {
                    allow: vec![],
                    deny: vec![],
                })
        };

        Some(RegexPageFilterRules {
            url: get("url"),
            owner: get("owner"),
            project: get("project"),
            channel: get("channel"),
            asset: get("asset"),
        })
    }
}

/// A [PageFilter] that imposes access control on pages by applying rules written in regular expressions.
pub struct RegexPageFilter {
    rules: RegexPageFilterRules,
}

impl RegexPageFilter {
    pub fn new(rules: RegexPageFilterRules) -> Self {
        Self { rules }
    }
}

impl PageFilter for RegexPageFilter {
    fn allow_url(&self, url: &url::Url) -> bool {
        self.rules.url.check(url.as_str())
    }

    fn allow_owner(&self, owner: &str) -> bool {
        self.rules.owner.check(owner)
    }

    fn allow_project(&self, project: &str) -> bool {
        self.rules.project.check(project)
    }

    fn allow_channel(&self, channel: &str) -> bool {
        self.rules.channel.check(channel)
    }

    fn allow_asset(&self, asset: &std::path::Path) -> bool {
        let asset = match asset.to_str() {
            Some(v) => v,
            None => {
                return false;
            }
        };

        self.rules.asset.check(asset)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use config::{Config, File, FileFormat};
    use url::Url;

    use crate::resolution::{PageFilter, RegexPageFilter, regex::RegexPageFilterRules};

    fn build_config(toml: &str) -> Config {
        Config::builder()
            .add_source(File::from_str(toml, FileFormat::Toml))
            .build()
            .unwrap()
    }

    #[test]
    fn test_allow_and_deny_combination() {
        let toml = r#"
    [security.filter]
    url = { allow = "example\\.com", deny = "forbidden" }
    "#;

        let cfg = build_config(toml);
        let filter = RegexPageFilterRules::from_config(&cfg).unwrap();

        let pf = RegexPageFilter::new(filter);

        let allowed_url = Url::parse("https://example.com/page").unwrap();
        let denied_url = Url::parse("https://example.com/forbidden").unwrap();
        let other_url = Url::parse("https://other.com").unwrap();

        assert!(pf.allow_url(&allowed_url)); // matches allow, not deny
        assert!(!pf.allow_url(&denied_url)); // matches deny
        assert!(!pf.allow_url(&other_url)); // not in allow
    }

    #[test]
    fn test_asset_filter_behavior() {
        let toml = r#"
    [security.filter]
    asset = { allow = ".*\\.(png|jpg)$", deny = "private_.*" }
    "#;

        let cfg = build_config(toml);
        let filter = RegexPageFilterRules::from_config(&cfg).unwrap();

        let pf = RegexPageFilter::new(filter);

        assert!(pf.allow_asset(Path::new("photo.png")));
        assert!(pf.allow_asset(Path::new("image.jpg")));
        assert!(!pf.allow_asset(Path::new("private_photo.png")));
        assert!(!pf.allow_asset(Path::new("document.pdf")));
    }

    #[test]
    fn test_owner_and_project_allow_all_when_empty() {
        let toml = r#"
    [security.filter]
    "#;

        let cfg = build_config(toml);
        let filter = RegexPageFilterRules::from_config(&cfg).unwrap();
        let pf = RegexPageFilter::new(filter);

        assert!(pf.allow_owner("anyone"));
        assert!(pf.allow_project("anything"));
    }

    #[test]
    fn test_array_allow_multiple_patterns() {
        let toml = r#"
    [security.filter]
    channel = ["alpha", "beta"]
    "#;

        let cfg = build_config(toml);
        let filter = RegexPageFilterRules::from_config(&cfg).unwrap();
        let pf = RegexPageFilter::new(filter);

        assert!(pf.allow_channel("alpha"));
        assert!(pf.allow_channel("beta"));
        assert!(!pf.allow_channel("gamma"));
    }

    #[test]
    fn test_combined_config_all_fields() {
        let toml = r#"
    [security.filter]
    url = "example\\.org"
    owner = { allow = "alice", deny = "bob" }
    project = { allow = "proj.*", deny = "secret" }
    channel = ["stable", "dev"]
    asset = { allow = ".*\\.rs$", deny = "test_.*" }
    "#;

        let cfg = build_config(toml);
        let filter = RegexPageFilterRules::from_config(&cfg).unwrap();
        let pf = RegexPageFilter::new(filter);

        let url = Url::parse("https://example.org/path").unwrap();
        assert!(pf.allow_url(&url));

        assert!(pf.allow_owner("alice"));
        assert!(!pf.allow_owner("bob"));

        assert!(pf.allow_project("proj_main"));
        assert!(!pf.allow_project("proj_secret"));

        assert!(pf.allow_channel("stable"));
        assert!(!pf.allow_channel("beta"));

        assert!(pf.allow_asset(Path::new("main.rs")));
        assert!(!pf.allow_asset(Path::new("test_main.rs")));
    }
}
