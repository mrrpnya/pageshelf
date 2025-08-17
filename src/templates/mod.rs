/// Utilities for handling [MiniJinja](https://docs.rs/minijinja/latest/minijinja/) templates.

use log::{error, info};
use minijinja::Environment;
use serde::{Deserialize, Serialize};

/// Identifier for the 404 template.
pub const TEMPLATE_404: &str = "404.html";
/// Identifier for the Index template.
pub const TEMPLATE_INDEX: &str = "index.html";

#[derive(Serialize)]
pub struct TemplateServerContext {
    pub name: String,
    pub about: String,
    pub home_url: Option<String>,
    pub icon_url: Option<String>,
    pub default_branch: String,
    pub version: &'static str,
}

#[derive(Serialize, Deserialize)]
pub struct TemplatePageContext {
    pub owner: String,
    pub repo: String,
}

#[derive(Serialize, Deserialize)]
pub struct TemplateErrorContext {
    pub code: u16,
    pub message: String,
}

/// Add a template to an environment.
/// Logs whether it succeeds or fails.
fn checked_add_template<'a>(env: &mut Environment<'a>, entry: &'a str, data: &'a str) {
    match env.add_template(entry, data) {
        Ok(_) => {
            info!("Added template {}", entry)
        }
        Err(e) => {
            error!("Error adding template for \"{}\": {}", entry, e)
        }
    }
}

/// Generates a MiniJinja environment from built-in resources.
/// This will include various pages off the bat.
pub fn templates_from_builtin<'a>() -> Environment<'a> {
    let mut env = Environment::new();

    // Styles
    checked_add_template(&mut env, "styles.css", include_str!("styles.css"));

    // Pages
    checked_add_template(&mut env, TEMPLATE_404, include_str!("404.jinja"));
    checked_add_template(&mut env, TEMPLATE_INDEX, include_str!("index.jinja"));
    checked_add_template(&mut env, "footer.html", include_str!("footer.jinja"));
    checked_add_template(&mut env, "header.html", include_str!("header.jinja"));

    env
}
