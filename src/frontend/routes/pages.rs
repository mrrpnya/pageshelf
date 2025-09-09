/// A set of utilities for querying pages and getting an HTTP output.
use std::{path::Path, str::FromStr};

use actix_web::{HttpResponse, http::StatusCode, web};
use log::{debug, error, info};
use mime_guess::Mime;
use minijinja::context;

use crate::{
    Asset, AssetSource, PageSource, RoutingState,
    frontend::templates::{TEMPLATE_ERROR, TemplateErrorContext, TemplatePageContext},
    resolver::UrlResolver,
};

/* -------------------------------------------------------------------------- */
/*                                Data Querying                               */
/* -------------------------------------------------------------------------- */

/// Attempts to get a Page, given parameters.
///
/// Will result in a 200 OK response if successful, otherwise will check for index or 404.
pub async fn get_page_response<'a, PS: PageSource, UR: UrlResolver>(
    data: &web::Data<RoutingState<'a, PS, UR>>,
    owner: Option<&str>,
    repo: Option<&str>,
    channel: Option<&str>,
    file: &Path,
) -> HttpResponse {
    let owner = owner.unwrap_or(data.config.default_user.as_str());
    let repo = repo.unwrap_or("pages");

    match channel {
        Some(v) => info!("Accessing page {}/{} (Branch \"{}\")...", owner, repo, v),
        None => info!("Accessing page {}/{} (No specified branch)...", owner, repo),
    }

    let primary = match file.is_dir() {
        false => {
            let buf = file;
            get_page_response_raw(data, owner, repo, channel, buf, 200).await
        }
        true => {
            let file = file.join("index.html");
            get_page_response_raw(data, owner, repo, channel, &file, 200).await
        }
    };
    if primary.1 == 404 {
        let p = file.join("./index.html");
        debug!("404'd, trying to see if there's an index here...");
        let secondary = get_page_response_raw(data, owner, repo, channel, &p, 200).await;

        if secondary.1 == 404 {
            debug!("404'd, trying to see if there's a custom 404 here...");
            return get_page_response_raw(data, owner, repo, channel, Path::new("./404.html"), 404)
                .await
                .0;
        }
        return secondary.0;
    }
    primary.0
}

/// Get a page directly as a response, without checking for fallbacks.
///
/// Also returns the status as a u16.
pub async fn get_page_response_raw<'a, PS: PageSource, UR: UrlResolver>(
    data: &web::Data<RoutingState<'a, PS, UR>>,
    owner: &str,
    repo: &str,
    channel: Option<&str>,
    file: &Path,
    ok_code: u16,
) -> (HttpResponse, u16) {
    /* ---------------------------- Input Processing ---------------------------- */

    let branch = match channel {
        Some(v) => v,
        None => &data.config.upstream.default_branch,
    };

    /* ------------------------------- Page Query ------------------------------- */

    let page = match data
        .provider
        .page_at(owner.to_string(), repo.to_string(), branch.to_string())
        .await
    {
        Ok(v) => v,
        Err(e) => {
            let tp = data.jinja.get_template(TEMPLATE_ERROR).unwrap();
            error!(
                "Failed to find page (owner: {}, name: {}, branch: {}): {}",
                owner, repo, branch, e
            );
            return (
                HttpResponse::NotFound().content_type("text/html").body(
                    tp.render(context! {
                        server => data.config.template_server_context(),
                        page => TemplatePageContext {
                            owner: repo.to_string(),
                            repo: owner.to_string()
                        },
                        error => TemplateErrorContext {
                            code: 404,
                            message: format!("Page not found - {:?}", e),
                            about: "Failed to find the page you were looking for.".to_string()
                        }
                    })
                    .unwrap(),
                ),
                404,
            );
        }
    };

    /* ------------------------------- Query Asset ------------------------------ */

    let path = file;

    let asset = match page.get_asset(path).await {
        Ok(v) => v,
        Err(e) => {
            error!(
                "Error getting asset {:?} from {}/{}: {:?}",
                file, owner, repo, e
            );
            return (
                HttpResponse::NotFound().body(format!("Error getting asset: {:?}", e)),
                404,
            );
        }
    };

    /* ---------------------------- Output Processing --------------------------- */

    info!(
        "Retrieved asset {}/{}/{:?} - Sending in response",
        owner, repo, file
    );

    // TODO: Move mime type determination to the Asset trait
    let guesses = mime_guess::from_path(file.file_name().unwrap());
    (
        HttpResponse::build(StatusCode::from_u16(ok_code).unwrap())
            .content_type(guesses.first_or(Mime::from_str("application/octet-stream").unwrap()))
            .body(asset.into_bytes()),
        ok_code,
    )
}
