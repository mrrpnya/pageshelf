pub mod response;

use std::{
    path::{Path, PathBuf},
    str::FromStr,
    time::Instant,
};

use color_eyre::eyre;
use mime_guess::Mime;
use tracing::{Level, debug, error, info, span};
use url::Url;

use crate::{
    Asset, AssetError, AssetSource,
    ext::Normalizable,
    frontend::{
        renderer::{FrontendErrorInfo, FrontendRenderer},
        response::{FrontendResponse, FrontendResponseBuilder},
    },
    project::{Page, Project, ProjectOwner, source::ProjectSource},
    resolver::{UrlResolution, UrlResolver},
};

pub mod renderer;

pub trait Frontend {
    async fn request_url<R: FrontendResponse>(&self, url: &Url) -> R;
}

/* -------------------------------------------------------------------------- */
/*                       Default Frontend implementation                      */
/* -------------------------------------------------------------------------- */

pub struct DefaultFrontend<R: FrontendRenderer, RR: UrlResolver, PS: ProjectSource> {
    pub renderer: R,
    pub url_resolver: RR,
    pub project_source: PS,
}

impl<R: FrontendRenderer, RR: UrlResolver, PS: ProjectSource> DefaultFrontend<R, RR, PS> {
    pub fn new(renderer: R, url_resolver: RR, page_source: PS) -> Self {
        Self {
            renderer,
            url_resolver,
            project_source: page_source,
        }
    }

    async fn request_asset<FR: FrontendResponse, P: Page>(&self, page: &P, asset: String) -> FR {
        let span = span!(Level::INFO, "request_asset");

        let _span = span.enter();
        let begin = Instant::now();
        debug!("Found page - Looking for asset...");
        let asset = Path::new(&asset);
        let mut assets = vec![asset];
        let local_index_path_buf = PathBuf::from(asset).join("index.html");
        let local_index_path = local_index_path_buf.as_path();
        assets.push(local_index_path);
        let local_404_path_buf = PathBuf::from(asset).join("./404.html");
        assets.push(local_404_path_buf.as_path());
        let local_404_path_buf = PathBuf::from(asset).join("../404.html");
        assets.push(local_404_path_buf.as_path());

        match page.get_first_asset_in(&assets).await {
            Ok((idx, asset)) => {
                let path = &assets[idx];
                let mut missing = false;
                if idx != 0 && path.ends_with("404.html") {
                    missing = true;
                }

                info!("[{}us] Found asset", begin.elapsed().as_micros());
                let mime = asset
                    .mime_type()
                    .map(|f| f.to_string())
                    .unwrap_or(
                        mime_guess::from_path(
                            path.file_name().unwrap(), // TODO: Remove this .unwrap()
                        )
                        .first_or(Mime::from_str("application/octet-stream").unwrap())
                        .to_string(),
                    )
                    .to_string();
                FR::status(match missing {
                    false => 200,
                    true => 404,
                })
                .content_type(&mime)
                .body(asset.bytes())
            }
            Err(AssetError::NotFound) => {
                let error_info = FrontendErrorInfo::status(404);
                self.renderer.render_error::<FR>(&error_info)
            }
            Err(e) => {
                // Don't try any more assets, they'll probably not work.
                let msg = format!("Error when attempting to get asset at {:?}: {:?}", asset, e);
                debug!("{}", msg);
                FR::internal_server_error()
                    .content_type("text/html")
                    .body(msg.as_bytes())
            }
        }
    }

    async fn request_page<FR: FrontendResponse>(
        &self,
        owner: String,
        project: String,
        channel: String,
        asset: String,
    ) -> FR {
        match self.project_source.get_owner(&owner).await {
            Ok(Some(owner)) => {
                debug!("Found owner - Looking for project...");
                match owner.get_project(&project).await {
                    Ok(Some(project)) => {
                        debug!("Found project - Looking for page...");
                        match project.get_channel(&channel).await {
                            Ok(Some(channel)) => self.request_asset(&channel, asset).await,
                            Ok(None) => {
                                let error_info = FrontendErrorInfo::status(404)
                                    .with_summary("Channel Not Found");
                                self.renderer.render_error::<FR>(&error_info)
                            }
                            Err(_) => {
                                let error_info = FrontendErrorInfo::status(500);
                                self.renderer.render_error::<FR>(&error_info)
                            }
                        }
                    }
                    Ok(None) => {
                        let error_info =
                            FrontendErrorInfo::status(404).with_summary("Project Not Found");
                        self.renderer.render_error::<FR>(&error_info)
                    }
                    Err(e) => {
                        let error_info = FrontendErrorInfo::page_error(&e);
                        self.renderer.render_error::<FR>(&error_info)
                    }
                }
            }
            Ok(None) => {
                let error_info = FrontendErrorInfo::status(404).with_summary("Owner Not Found");
                self.renderer.render_error::<FR>(&error_info)
            }
            Err(e) => {
                let error_info = FrontendErrorInfo::page_error(&e);
                self.renderer.render_error::<FR>(&error_info)
            }
        }
    }
}

impl<R: FrontendRenderer, RR: UrlResolver, PS: ProjectSource> Frontend
    for DefaultFrontend<R, RR, PS>
{
    async fn request_url<FR: FrontendResponse>(&self, url: &Url) -> FR {
        let span = span!(Level::INFO, "request_url");

        // TODO: Separate logic and rendering more?

        let _span = span.enter();

        if let Some(custom) = self.renderer.render_custom::<FR>(url) {
            return custom;
        }

        match self.url_resolver.resolve(url) {
            Ok(resolution) => {
                let span = span!(Level::DEBUG, "resolution", resolution = ?resolution);

                let _span = span.enter();
                match resolution {
                    UrlResolution::Index => self.renderer.render_index::<FR>(),
                    UrlResolution::Page {
                        owner,
                        name,
                        branch,
                        asset,
                    } => self.request_page(owner, name, branch, asset).await,
                    UrlResolution::Domain { domain, asset } => {
                        match self.project_source.resolve_domain(&domain).await {
                            Ok(domains) => {
                                // Make iterator peekable to check emptiness without consuming
                                let mut domains = domains.peekable();
                                if domains.peek().is_none() {
                                    debug!("Could not find domain");
                                    let error_info = FrontendErrorInfo::status(404)
                                        .with_summary("Domain Not Found");
                                    self.renderer.render_error::<FR>(&error_info)
                                } else {
                                    // Take the first resolved domain
                                    if let Some(domain_page) = domains.next() {
                                        let owner = domain_page.owner;
                                        let name = domain_page.project;
                                        let branch = domain_page.channel; // assuming channel maps to branch
                                        self.request_page(owner, name, branch, asset).await
                                    } else {
                                        // This case is unlikely due to peek check, but safer to handle
                                        debug!("Domain iterator unexpectedly empty");
                                        let error_info = FrontendErrorInfo::status(500)
                                            .with_summary("Domain Not Found");
                                        self.renderer.render_error::<FR>(&error_info)
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Error finding domain: {:?}", e);
                                let error_info = FrontendErrorInfo::status(500)
                                    .with_summary("Error Finding Domain");
                                self.renderer.render_error::<FR>(&error_info)
                            }
                        }
                    }
                }
            }
            Err(url_error) => {
                error!("Failed to resolve URL: {url_error:?}");
                let error_info = FrontendErrorInfo::url_resolution_error(&url_error);
                self.renderer.render_error::<FR>(&error_info)
            }
        }
    }
}
