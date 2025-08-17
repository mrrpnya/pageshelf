/// Default Actix routes for querying pages.

use std::str::FromStr;

use actix_web::{HttpResponse, Responder, web};
use log::{error, info};
use mime_guess::Mime;
use minijinja::context;

use crate::{
    asset::{Asset, AssetPath, AssetQueryable},
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

    get_page_master(data, owner, repo, None, file).await
}

/// Page query route (Owner-Repo-Branch-File)
pub async fn get_page_orbf<'a, PS: PageSource>(
    path: web::Path<(String, String, String, String)>,
    data: web::Data<RouteSharedData<'a, PS>>,
) -> impl Responder {
    let (owner, repo, branch, file) = path.into_inner();

    let branch = branch.clone();

    get_page_master(data, owner, repo, Some(&branch), file).await
}

/// Page query route (Owner-Repo)
pub async fn get_page_or<'a, PS: PageSource>(
    path: web::Path<(String, String)>,
    data: web::Data<RouteSharedData<'a, PS>>,
) -> impl Responder {
    let (owner, repo) = path.into_inner();

    get_page_master(data, owner, repo, None, "index.html".to_string()).await
}

/// Page query route (Owner-Repo-Branch)
pub async fn get_page_orb<'a, PS: PageSource>(
    path: web::Path<(String, String, String)>,
    data: web::Data<RouteSharedData<'a, PS>>,
) -> impl Responder {
    let (owner, repo, branch) = path.into_inner();

    let branch = branch.clone();

    get_page_master(data, owner, repo, Some(&branch), "index.html".to_string()).await
}

/* -------------------------------------------------------------------------- */
/*                                Data Querying                               */
/* -------------------------------------------------------------------------- */

/// Base action for querying a page via the web.
pub async fn get_page_master<'a, PS: PageSource>(
    data: web::Data<RouteSharedData<'a, PS>>,
    owner: String,
    repo: String,
    channel: Option<&str>,
    file: String,
) -> impl Responder + use<PS> {
    /* ---------------------------- Input Processing ---------------------------- */

    match channel {
        Some(v) => info!("Accessing page {}/{} (Branch \"{}\")...", owner, repo, v),
        None => info!("Accessing page {}/{} (No specified branch)...", owner, repo),
    }

    let branch = match channel {
        Some(v) => v,
        None => &data.config.upstream.default_branch,
    };

    /* ------------------------------- Page Query ------------------------------- */

    let page = match data.provider.page_at(&owner, &repo, &branch).await {
        Ok(v) => v,
        Err(e) => {
            let tp = data.jinja.get_template(TEMPLATE_404).unwrap();
            error!("Failed to find page (owner: {}, name: {}, branch: {}): {}", owner, repo, branch, e);
            return HttpResponse::NotFound().body(
                tp.render(context! {
                    server => data.config.template_server_context(),
                    page => TemplatePageContext {
                        owner: repo.clone(),
                        repo: owner.clone()
                    },
                    error => TemplateErrorContext {
                        code: 404,
                        message: format!("Page not found - {:?}", e)
                    }
                })
                .unwrap(),
            );
        }
    };

    // Page is found - Log that we're getting the asset
    match channel {
        Some(v) => info!(
            "Retrieving asset {} from page {}/{} (Branch \"{}\")...",
            file, owner, repo, v
        ),
        None => info!(
            "Retrieving asset {} from page {}/{} (No specified branch)...",
            file, owner, repo
        ),
    }

    /* ------------------------------- Query Asset ------------------------------ */
    
    let path = AssetPath::from_str(file.as_str()).unwrap();

    let asset = match page.asset_at(&path).await {
        Ok(v) => v,
        Err(e) => {
            error!(
                "Error getting asset {} from {}/{}: {:?}",
                file, owner, repo, e
            );
            return HttpResponse::NotFound().body(format!("Error getting asset: {:?}", e));
        }
    };

    /* ---------------------------- Output Processing --------------------------- */

    info!(
        "Retrieved asset {}/{}/{} - Sending in response",
        owner, repo, file
    );

    // TODO: Move mime type determination to the Asset trait
    let guesses = mime_guess::from_path(file);
    HttpResponse::Ok()
        .content_type(guesses.first_or(Mime::from_str("application/octet-stream").unwrap()))
        .body(asset.body())
}