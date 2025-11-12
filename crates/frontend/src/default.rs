use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use metrics::counter;
use mime_guess::mime::APPLICATION_OCTET_STREAM;
use tracing::{Level, error, info, span};
use url::Url;

use pageshelf_core::{
    ext::Normalizable,
    resolution::PageResolver,
    upstream::{Upstream, UpstreamError},
};

use crate::{
    Frontend,
    renderer::{FrontendErrorInfo, Renderer},
    response::{FrontendResponse, FrontendResponseBuilder},
};

pub struct DefaultFrontend<R: Renderer, PR: PageResolver, US: Upstream> {
    pub renderer: R,
    pub resolver: PR,
    pub upstream: Arc<US>,
}

impl<R: Renderer, PR: PageResolver, US: Upstream> DefaultFrontend<R, PR, US> {
    pub fn new(renderer: R, resolver: PR, upstream: Arc<US>) -> Self {
        Self {
            renderer,
            resolver,
            upstream,
        }
    }

    async fn request_asset<FR: FrontendResponse>(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        path: &Path,
    ) -> FR {
        let span = span!(Level::INFO, "request_asset");
        let _span = span.enter();

        let begin = Instant::now();

        fn respond_with_asset<FR2: FrontendResponse>(path: &Path, data: &[u8]) -> FR2 {
            let mime = match path.file_name() {
                Some(v) => mime_guess::from_path(v).first_or(APPLICATION_OCTET_STREAM),
                None => APPLICATION_OCTET_STREAM,
            };

            FR2::ok().content_type(mime.as_ref()).build(data)
        }

        let mut assets = Vec::new();
        let rel = path.normalized_relative(); // Leading slash removed here
        assets.push(rel.as_path());
        let local_index_path_buf = PathBuf::from(path)
            .join("./index.html")
            .normalized_relative();
        let local_index_path = local_index_path_buf.as_path();
        assets.push(local_index_path);
        let local_404_path_buf = PathBuf::from(path).join("./404.html").normalized_relative();
        assets.push(local_404_path_buf.as_path());
        let local_404_path_buf = PathBuf::from(path)
            .join("../404.html")
            .normalized_relative();
        assets.push(local_404_path_buf.as_path());

        match self
            .upstream
            .get_first_asset_bytes(owner, project, channel, &assets)
            .await
        {
            Ok((index, data)) => {
                counter!("asset.upstream.found").increment(1);
                info!("[{}us] Found asset", begin.elapsed().as_micros());
                respond_with_asset(assets[index], &data)
            }
            Err(UpstreamError::NotFound) => {
                counter!("asset.upstream.not_found").increment(1);
                let error_info = FrontendErrorInfo::status(404).with_summary("Asset not found");
                self.renderer.render_error::<FR>(&error_info)
            }
            Err(e) => {
                counter!("asset.upstream.error").increment(1);
                error!("Error getting asset: {e:?}");
                let error_info = FrontendErrorInfo::status(500);
                self.renderer.render_error::<FR>(&error_info)
            }
        }
    }
}

impl<R: Renderer, PR: PageResolver, US: Upstream> Frontend for DefaultFrontend<R, PR, US> {
    async fn request_url<FR: FrontendResponse>(&self, url: &Url) -> FR {
        let span = span!(Level::INFO, "request_url");

        // TODO: Separate logic and rendering more?

        let _span = span.enter();

        if let Some(custom) = self.renderer.render_custom::<FR>(url) {
            return custom;
        }

        match self.resolver.resolve_url(url).await {
            Ok(res) => {
                let owner = res.owner();
                let project = res.project();
                let channel = res.channel();
                let path = res.path();
                let span = span!(
                    Level::DEBUG,
                    "resolved",
                    page.owner = ?owner,
                    page.project = ?project,
                    page.channel = ?channel,
                    page.path = ?path
                );

                let _span = span.enter();
                self.request_asset(owner, project, channel, path).await
            }
            Err(url_error) => {
                error!("Failed to resolve URL: {url_error:?}");
                let error_info = FrontendErrorInfo::resolution_error(&url_error);
                self.renderer.render_error::<FR>(&error_info)
            }
        }
    }
}
