use minijinja::Environment;
use serde::{Deserialize, Serialize};
/// Utilities for handling [MiniJinja](https://docs.rs/minijinja/latest/minijinja/) templates.
use tracing::{debug, error};

/* -------------------------------------------------------------------------- */
/*                           Known page identifiers                           */
/* -------------------------------------------------------------------------- */

/// Identifier for the Error template.
pub const TEMPLATE_ERROR: &str = "error.html";
/// Identifier for the Index template.
pub const TEMPLATE_INDEX: &str = "index.html";

/* -------------------------------------------------------------------------- */
/*                             Rendering contexts                             */
/* -------------------------------------------------------------------------- */

#[derive(Serialize, Clone)]
pub struct TemplateServerContext {
    pub name: String,
    pub about: String,
    pub domain: Option<String>,
    pub icon_url: Option<String>,
    pub default_branch: String,
    pub version: &'static str,
}

#[derive(Serialize, Deserialize)]
pub struct TemplatePageContext<'a> {
    pub owner: &'a str,
    pub repo: &'a str,
}

#[derive(Serialize, Deserialize)]
pub struct TemplateErrorContext<'a> {
    pub code: Option<u16>,
    pub summary: Option<&'a str>,
    pub details: Option<&'a str>,
    pub suggestions: Option<Vec<&'a str>>,
    pub notes: Option<Vec<&'a str>>,
}

/* -------------------------------------------------------------------------- */
/*                                    Setup                                   */
/* -------------------------------------------------------------------------- */

/// Add a template to an environment.
/// Logs whether it succeeds or fails.
fn checked_add_template<'a>(env: &mut Environment<'a>, entry: &'a str, data: &'a str) {
    match env.add_template(entry, data) {
        Ok(_) => {
            debug!("Added template {}", entry)
        }
        Err(e) => {
            error!("Error adding template for \"{}\": {}", entry, e)
        }
    }
}

/// Generates a MiniJinja environment from built-in resources.
/// This will include various pages off the bat.
pub fn env_from_builtin() -> Environment<'static> {
    let mut env = Environment::new();

    // Styles
    checked_add_template(&mut env, "styles.css", include_str!("styles.css"));

    // Pages
    checked_add_template(&mut env, TEMPLATE_ERROR, include_str!("error.jinja"));
    checked_add_template(&mut env, TEMPLATE_INDEX, include_str!("index.jinja"));
    checked_add_template(&mut env, "footer.html", include_str!("footer.jinja"));
    checked_add_template(&mut env, "header.html", include_str!("header.jinja"));

    env
}
