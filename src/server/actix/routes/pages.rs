/// A set of utilities for querying pages and getting an HTTP output.
use std::{path::Path, str::FromStr};

use actix_web::{HttpResponse, http::StatusCode, web};
use log::{debug, error, info};
use mime_guess::Mime;

use crate::{
    Asset, AssetSource,
    frontend::{
        Frontend,
        renderer::{FrontendErrorInfo, FrontendRenderer},
    },
    page::source::PageSource,
    resolver::UrlResolver,
    server::actix::routes::RoutingState,
};

/* -------------------------------------------------------------------------- */
/*                                Data Querying                               */
/* -------------------------------------------------------------------------- */

/// Attempts to get a Page, given parameters.
///
/// Will result in a 200 OK response if successful, otherwise will check for index or 404.
pub async fn get_page_response<PS: PageSource, UR: UrlResolver, RD: FrontendRenderer>(
    data: &web::Data<RoutingState<PS, UR, RD>>,
    owner: &str,
    repo: &str,
    branch: &str,
    file: &Path,
) -> HttpResponse {
    let primary = match file.is_dir() {
        false => {
            let buf = file;
            get_page_response_raw(data, owner, repo, branch, buf, 200).await
        }
        true => {
            let file = file.join("index.html");
            get_page_response_raw(data, owner, repo, branch, &file, 200).await
        }
    };
    if primary.1 == 404 {
        let p = file.join("./index.html");
        debug!("404'd, trying to see if there's an index here...");
        let secondary = get_page_response_raw(data, owner, repo, branch, &p, 200).await;

        if secondary.1 == 404 {
            debug!("404'd, trying to see if there's a custom 404 here...");
            return get_page_response_raw(data, owner, repo, branch, Path::new("./404.html"), 404)
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
pub async fn get_page_response_raw<F: Frontend>(
    data: &web::Data<RoutingState<F>>,
    owner: &str,
    repo: &str,
    branch: &str,
    file: &Path,
    ok_code: u16,
) -> (HttpResponse, u16) {
    /* ------------------------------- Page Query ------------------------------- */

    let page = match data
        .provider
        .page_at(owner.to_string(), repo.to_string(), branch.to_string())
        .await
    {
        Ok(v) => v,
        Err(e) => {
            error!(
                "Failed to find page (owner: {}, name: {}, branch: {}): {}",
                owner, repo, branch, e
            );
            let info = FrontendErrorInfo::page_error(&e)
                .with_owner(owner)
                .with_repo(repo)
                .with_branch(branch);
            let render = data.renderer.render_error::<HttpResponse>(&info);
            return (render, 404);
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
