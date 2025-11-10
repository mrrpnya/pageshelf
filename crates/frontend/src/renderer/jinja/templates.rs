use color_eyre::eyre::{self, Context, eyre};
use include_dir::{Dir, DirEntry, include_dir};
use minijinja::Environment;
use serde::{Deserialize, Serialize};
/// Utilities for handling [MiniJinja](https://docs.rs/minijinja/latest/minijinja/) templates.
use tracing::{debug, info};

/* -------------------------------------------------------------------------- */
/*                           Known page identifiers                           */
/* -------------------------------------------------------------------------- */

pub static TEMPLATE_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/templates/");
pub static STATIC_ASSET_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/static/");

/// Identifier for the Error template.
pub const TEMPLATE_ERROR: &str = "error.jinja";
/// Identifier for the Index template.
pub const TEMPLATE_INDEX: &str = "index.jinja";

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

#[derive(Serialize, Deserialize, Default)]
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
fn checked_add_template<'a>(
    env: &mut Environment<'a>,
    entry: &'a str,
    data: &'a str,
) -> Result<(), eyre::Report> {
    env.add_template(entry, data)
        .wrap_err_with(|| format!("Failed to add template '{entry}'"))
}

/// Creates a MiniJinja environment from embedded templates.
pub fn env_from_builtin() -> Result<Environment<'static>, eyre::Report> {
    let span = tracing::info_span!("load_templates");
    let _enter = span.enter();

    let mut env = Environment::new();
    let mut count = 0;

    for entry in TEMPLATE_DIR.entries() {
        if let DirEntry::File(file) = entry {
            let path = file.path().to_string_lossy().to_string();

            // SAFETY: We intentionally leak the string to satisfy 'static lifetimes.
            let path_static: &'static str = Box::leak(path.into_boxed_str());

            let data = file
                .contents_utf8()
                .ok_or_else(|| eyre!("Non-UTF8 template: {}", path_static))?;

            checked_add_template(&mut env, path_static, data)?;
            debug!("Loaded template '{}'", path_static);
            count += 1;
        }
    }

    info!("Loaded {} templates into MiniJinja environment", count);
    Ok(env)
}
