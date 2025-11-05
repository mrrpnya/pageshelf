//! Provides a MiniJinja-based implementation of a FrontendRenderer.

use clap::crate_version;
use minijinja::{Environment, Template, context};
use serde::Serialize;

use crate::{
    conf::ServerConfig,
    frontend::{
        renderer::{
            FrontendErrorInfo, FrontendRenderer,
            jinja::templates::{
                TEMPLATE_ERROR, TEMPLATE_INDEX, TemplateErrorContext, TemplatePageContext,
                TemplateServerContext,
            },
        },
        response::{FrontendResponse, FrontendResponseBuilder},
    },
};

const LOGO_WEBP: &[u8] = std::include_bytes!("../../../../branding/pageshelf_logo.webp");

use tracing::error;

mod templates;

#[derive(Debug)]
enum JinjaFrontendRendererError {
    RenderFail(minijinja::Error),
    MissingTemplate(minijinja::Error),
}

#[derive(Clone)]
pub struct JinjaFrontendRenderer {
    env: Environment<'static>,
    tp_serv_ctx: TemplateServerContext,
}

impl JinjaFrontendRenderer {
    pub fn new(
        env: Environment<'static>,
        name: String,
        about: String,
        domain: Option<String>,
        icon_url: Option<String>,
        default_branch: String,
    ) -> Self {
        Self {
            env,
            tp_serv_ctx: TemplateServerContext {
                name,
                about,
                domain,
                icon_url,
                default_branch,
                version: crate_version!(),
            },
        }
    }

    pub fn from_server_config(config: &ServerConfig) -> Self {
        let env = templates::env_from_builtin();
        Self::new(
            env,
            config.name.to_string(),
            config.description.to_string(),
            config.domain.as_ref().map(|v| v.as_str().to_string()),
            Some("/pages_favicon.webp".to_string()),
            config.upstream.default_branch.clone(),
        )
    }

    fn try_render_template<R: FrontendResponse, S: Serialize>(
        &self,
        code: u16,
        template_name: &str,
        context: S,
    ) -> Result<R, JinjaFrontendRendererError> {
        let tp: Result<Template<'_, '_>, minijinja::Error> = self.env.get_template(template_name);
        match tp {
            Ok(tp) => {
                let body = tp.render(context);
                match body {
                    Ok(body) => Ok(R::status(code)
                        .content_type("text/html")
                        .body(body.as_bytes())),
                    Err(e) => {
                        error!("Failed to render Jinja template \"{template_name}\": {e}");
                        Err(JinjaFrontendRendererError::RenderFail(e))
                    }
                }
            }
            Err(e) => {
                error!("Error getting Jinja template \"{template_name}\": {e}");
                Err(JinjaFrontendRendererError::MissingTemplate(e))
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

impl Default for JinjaFrontendRenderer {
    fn default() -> Self {
        let cfg = ServerConfig::default();

        Self::from_server_config(&cfg)
    }
}

impl FrontendRenderer for JinjaFrontendRenderer {
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
                    .body(body.as_bytes())
            }
        }
    }

    fn render_custom<R: FrontendResponse>(&self, url: &url::Url) -> Option<R> {
        if url.path() == "pageshelf_logo.webp" {
            return Some(R::ok().content_type("image/webp").body(LOGO_WEBP));
        }

        None
    }
}
