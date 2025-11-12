//! Provides a MiniJinja-based implementation of a Renderer.

use std::{collections::HashMap, error::Error, fmt::Display};

use clap::crate_version;
use include_dir::DirEntry;
use mime_guess::mime::APPLICATION_OCTET_STREAM;
use minijinja::{Environment, Template, context};
use serde::Serialize;

const LOGO_WEBP: &[u8] = std::include_bytes!("../../../../../branding/pageshelf_logo.webp");

use tracing::{error, info};

use crate::{
    renderer::{
        FrontendErrorInfo, Renderer,
        jinja::templates::{
            STATIC_ASSET_DIR, TEMPLATE_ERROR, TEMPLATE_INDEX, TemplateErrorContext,
            TemplatePageContext, TemplateServerContext, env_from_builtin,
        },
    },
    response::{FrontendResponse, FrontendResponseBuilder},
};

mod templates;

#[derive(Debug)]
enum JinjaRendererError {
    RenderFail(minijinja::Error),
    MissingTemplate(minijinja::Error),
}

impl Display for JinjaRendererError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JinjaRendererError::RenderFail(e) => {
                write!(f, "Rendering Failure: {e}")
            }
            JinjaRendererError::MissingTemplate(e) => {
                write!(f, "Missing Template: {e}")
            }
        }
    }
}

impl Error for JinjaRendererError {}

#[derive(Clone)]
pub struct JinjaRenderer {
    env: Environment<'static>,
    tp_serv_ctx: TemplateServerContext,
    static_assets: HashMap<&'static str, &'static str>,
}

impl JinjaRenderer {
    pub fn new(
        env: Option<Environment<'static>>,
        name: String,
        about: String,
        domain: Option<String>,
        icon_url: Option<String>,
        default_branch: String,
    ) -> Self {
        let mut static_assets = HashMap::default();
        for entry in STATIC_ASSET_DIR.entries() {
            if let DirEntry::File(file) = entry {
                static_assets.insert(file.path().to_str().unwrap(), file.contents_utf8().unwrap());
            }
        }

        info!("{} static Jinja assets registered", static_assets.len());

        Self {
            env: env.unwrap_or(env_from_builtin().unwrap()),
            tp_serv_ctx: TemplateServerContext {
                name,
                about,
                domain,
                icon_url: Some(
                    icon_url.unwrap_or("/_jinja_static/pageshelf_logo.webp".to_string()),
                ),
                default_branch,
                version: crate_version!(),
            },
            static_assets,
        }
    }

    fn try_render_template<R: FrontendResponse, S: Serialize>(
        &self,
        code: u16,
        template_name: &str,
        context: S,
    ) -> Result<R, JinjaRendererError> {
        let tp: Result<Template<'_, '_>, minijinja::Error> = self.env.get_template(template_name);
        match tp {
            Ok(tp) => {
                let body = tp.render(context);
                match body {
                    Ok(body) => Ok(R::status(code)
                        .content_type("text/html")
                        .build(body.as_bytes())),
                    Err(e) => {
                        error!("Failed to render Jinja template \"{template_name}\": {e}");
                        Err(JinjaRendererError::RenderFail(e))
                    }
                }
            }
            Err(e) => {
                error!("Error getting Jinja template \"{template_name}\": {e}");
                Err(JinjaRendererError::MissingTemplate(e))
            }
        }
    }

    fn render_template_handled<R: FrontendResponse, S: Serialize>(
        &self,
        code: u16,
        template_name: &str,
        context: S,
    ) -> R {
        match self.try_render_template::<R, S>(code, template_name, context) {
            Ok(response) => response,
            Err(e) => {
                let details = format!(
                    "An internal error occurred while rendering the template: {:?}",
                    e
                );
                let info = FrontendErrorInfo::status(500)
                    .with_summary("Internal Server Error")
                    .with_details(&details);
                self.render_error::<R>(&info)
            }
        }
    }
}

impl Default for JinjaRenderer {
    fn default() -> Self {
        let env = templates::env_from_builtin()
            .expect("Failed to initialize the built-in Jinja environment");
        Self::new(
            Some(env),
            "Pageshelf".to_string(),
            "A free and open-source Pages server".to_string(),
            Some("localhost".to_string()),
            Some("/_jinja_static/pageshelf_logo.webp".to_string()),
            "pages".to_string(),
        )
    }
}

impl Renderer for JinjaRenderer {
    fn render_index<R: FrontendResponse>(&self) -> R {
        self.render_template_handled::<R, _>(
            200,
            TEMPLATE_INDEX,
            context! {
                server => &self.tp_serv_ctx,
            },
        )
    }

    fn render_error<'a, R: FrontendResponse>(&self, info: &FrontendErrorInfo<'a>) -> R {
        match self.try_render_template(
            info.status_code.unwrap_or(500),
            TEMPLATE_ERROR,
            context! {
                server => &self.tp_serv_ctx,
                page => TemplatePageContext {
                    owner: info.owner.unwrap_or(""),
                    repo: info.repo.unwrap_or("")
                },
                error => TemplateErrorContext {
                    code: info.status_code,
                    summary: info.summary,
                    details: info.details,
                    suggestions: None,
                    notes: None,
                }
            },
        ) {
            Ok(response) => response,
            Err(e) => {
                let details = format!(
                    "An internal error occurred while rendering the error template: {:?}",
                    e
                );
                error!("{}", details);
                let fallback_info = FrontendErrorInfo::status(500)
                    .with_summary("Internal Server Error")
                    .with_details(&details);
                let body = format!(
                    "<html><head><title>{}</title></head><body><h1>{}</h1><p>{}</p></body></html>",
                    fallback_info.summary.unwrap_or("Internal Server Error"),
                    fallback_info.summary.unwrap_or("Internal Server Error"),
                    fallback_info
                        .details
                        .unwrap_or("An internal server error occurred.")
                );
                R::internal_server_error()
                    .content_type("text/html")
                    .build(body.as_bytes())
            }
        }
    }

    fn render_custom<R: FrontendResponse>(&self, url: &url::Url) -> Option<R> {
        if url.path().is_empty() || url.path() == "/" {
            match url.domain() {
                Some(url_domain) => match &self.tp_serv_ctx.domain {
                    Some(serv_domain) => {
                        if serv_domain == url_domain {
                            return Some(self.render_index::<R>());
                        }
                    }
                    None => {
                        return Some(self.render_index::<R>());
                    }
                },
                None => {
                    return Some(self.render_index::<R>());
                }
            }
        }
        let path = url.path().to_string();
        if url.path() == "/_jinja_static/pageshelf_logo.webp" {
            return Some(R::ok().content_type("image/webp").build(LOGO_WEBP));
        }

        if let Some(stripped) = path.strip_prefix("/_jinja_static/")
            && let Some(contents) = self.static_assets.get(stripped)
        {
            let mime = mime_guess::from_path(stripped)
                .first_or(APPLICATION_OCTET_STREAM)
                .to_string();

            return Some(R::ok().content_type(&mime).build(contents.as_bytes()));
        }

        None
    }
}
