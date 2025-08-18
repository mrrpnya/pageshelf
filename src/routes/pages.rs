/// Default Actix routes for querying pages.
use std::{path::Path, str::FromStr, time::SystemTime};

use actix_web::{HttpResponse, Responder, http::StatusCode, web};
use log::{error, info};
use mime_guess::Mime;
use minijinja::context;

use crate::{
    asset::{Asset, AssetQueryable},
    page::PageSource,
    routes::RouteSharedData,
    templates::{TEMPLATE_404, TemplateErrorContext, TemplatePageContext},
};

/* -------------------------------------------------------------------------- */
/*                               Exposed Queries                              */
/* -------------------------------------------------------------------------- */

// TODO: Consider renaming these?

/// Page query route (Owner-Repo-File)
pub async fn get_page_orf<'a, PS: PageSource>(
    path: web::Path<(String, String, String)>,
    data: web::Data<RouteSharedData<'a, PS>>,
) -> impl Responder {
    let (owner, repo, file) = path.into_inner();
    get_page(&data, &owner, &repo, None, Path::new(&file)).await
}

/// Page query route (Owner-Repo-Branch-File)
pub async fn get_page_orbf<'a, PS: PageSource>(
    path: web::Path<(String, String, String, String)>,
    data: web::Data<RouteSharedData<'a, PS>>,
) -> impl Responder {
    let (owner, repo, branch, file) = path.into_inner();

    let branch = branch.clone();

    get_page(&data, &owner, &repo, Some(&branch), Path::new(&file)).await
}

/// Page query route (Owner-Repo)
pub async fn get_page_or<'a, PS: PageSource>(
    path: web::Path<(String, String)>,
    data: web::Data<RouteSharedData<'a, PS>>,
) -> impl Responder {
    let (owner, repo) = path.into_inner();

    let p = std::path::absolute(Path::new("/")).unwrap();
    get_page(&data, &owner, &repo, None, &p).await
}

/// Page query route (Owner-Repo-Branch)
pub async fn get_page_orb<'a, PS: PageSource>(
    path: web::Path<(String, String, String)>,
    data: web::Data<RouteSharedData<'a, PS>>,
) -> impl Responder {
    let (owner, repo, branch) = path.into_inner();

    let branch = branch.clone();
    let p = std::path::absolute(Path::new("/")).unwrap();
    get_page(&data, &owner, &repo, Some(&branch), &p).await
}

/* -------------------------------------------------------------------------- */
/*                                Data Querying                               */
/* -------------------------------------------------------------------------- */

async fn get_page<'a, PS: PageSource>(
    data: &web::Data<RouteSharedData<'a, PS>>,
    owner: &str,
    repo: &str,
    channel: Option<&str>,
    file: &Path,
) -> impl Responder + use<PS> {
    match channel {
        Some(v) => info!("Accessing page {}/{} (Branch \"{}\")...", owner, repo, v),
        None => info!("Accessing page {}/{} (No specified branch)...", owner, repo),
    }

    let primary = match file.is_dir() {
        false => {
            let buf = file;
            get_page_raw(data, owner, repo, channel, &buf, 200).await
        }
        true => {
            let file = file.join("./index.html");
            get_page_raw(data, owner, repo, channel, &file, 200).await
        }
    };
    if primary.1 == 404 {
        /*let buf = match file.is_dir() {
            true => file.join(Path::new("./404.html")),
            false => match file.parent() {
                Some(v) => v.join(Path::new("./404.html")),
                None => file.join(Path::new("./404.html")),
            },
        };*/
        return get_page_raw(data, owner, repo, channel, Path::new("/404.html"), 404)
            .await
            .0;
    }
    primary.0
}

/// Base action for querying a page via the web.
pub async fn get_page_raw<'a, PS: PageSource>(
    data: &web::Data<RouteSharedData<'a, PS>>,
    owner: &str,
    repo: &str,
    channel: Option<&str>,
    file: &Path,
    ok_code: u16,
) -> (impl Responder + use<PS>, u16) {
    /* ---------------------------- Input Processing ---------------------------- */

    let branch = match channel {
        Some(v) => v,
        None => &data.config.upstream.default_branch,
    };

    /* ------------------------------- Page Query ------------------------------- */

    let page = match data.provider.page_at(&owner, &repo, &branch).await {
        Ok(v) => v,
        Err(e) => {
            let tp = data.jinja.get_template(TEMPLATE_404).unwrap();
            error!(
                "Failed to find page (owner: {}, name: {}, branch: {}): {}",
                owner, repo, branch, e
            );
            return (
                HttpResponse::NotFound().body(
                    tp.render(context! {
                        server => data.config.template_server_context(),
                        page => TemplatePageContext {
                            owner: repo.to_string(),
                            repo: owner.to_string()
                        },
                        error => TemplateErrorContext {
                            code: 404,
                            message: format!("Page not found - {:?}", e)
                        }
                    })
                    .unwrap(),
                ),
                404,
            );
        }
    };

    // Page is found - Log that we're getting the asset
    match channel {
        Some(v) => info!(
            "Retrieving asset {:?} from page {}/{} (Branch \"{}\")...",
            file, owner, repo, v
        ),
        None => info!(
            "Retrieving asset {:?} from page {}/{} (No specified branch)...",
            file, owner, repo
        ),
    }

    /* ------------------------------- Query Asset ------------------------------ */

    let path = file;

    let asset = match page.asset_at(&path).await {
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
            .body(asset.body()),
        ok_code,
    )
}
