use minijinja::Environment;
use serde::{Deserialize, Serialize};

pub const TEMPLATE_404: &str = "404.html";
pub const TEMPLATE_INDEX: &str = "index.html";

#[derive(Serialize, Deserialize)]
pub struct TemplateServerContext {
    pub name: String,
    pub about: String,
    pub home_url: Option<String>,
    pub icon_url: Option<String>,
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

pub fn templates_from_builtin<'a>() -> Environment<'a> {
    let mut env = Environment::new();

    env.add_template(TEMPLATE_404, include_str!("404.jinja"));
    env.add_template(TEMPLATE_INDEX, include_str!("index.jinja"));
    env.add_template("footer.html", include_str!("footer.jinja"));
    env.add_template("styles.css", include_str!("styles.css"));
    env.add_template("header.html", include_str!("header.jinja"));

    env
}