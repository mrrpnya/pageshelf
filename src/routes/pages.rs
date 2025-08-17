
use actix_web::{HttpResponse, Responder, get, http::StatusCode, web};
use log::{error, info};
use minijinja::context;

use crate::{
    PageSource,
    asset::{Asset, AssetPath, AssetQueryable},
    routes::RouteSharedData,
    templates::{TEMPLATE_404, TemplateErrorContext, TemplatePageContext, template_server_context},
};

async fn get_page_master<'a>(
    data: web::Data<RouteSharedData<'a>>,
    owner: String,
    repo: String,
    channel: Option<&str>,
    file: String,
) -> impl Responder + use<> {
    match channel {
        Some(v) => info!("Accessing page {}/{} (Branch \"{}\")...", owner, repo, v),
        None => info!("Accessing page {}/{} (No specified branch)...", owner, repo),
    }

    let page = match data.provider.page_at(&owner, &repo, &data.config.upstream.default_branch).await {
        Ok(v) => v,
        Err(e) => {
            let tp = data.jinja.get_template(TEMPLATE_404).unwrap();
            return HttpResponse::with_body(
                StatusCode::NOT_FOUND,
                tp.render(context! {
                    server => template_server_context(),
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

    // TODO: Remove this unwrap
    let path = AssetPath::try_from(file.as_str()).unwrap();

    let asset = match page.asset_at(&path).await {
        Ok(v) => v,
        Err(e) => {
            error!(
                "Error getting asset {} from {}/{}: {:?}",
                file, owner, repo, e
            );
            return HttpResponse::with_body(
                StatusCode::NOT_FOUND,
                format!("Error getting asset: {:?}", e),
            );
        }
    };

    info!(
        "Retrieved asset {}/{}/{} - Sending in response",
        owner, repo, file
    );

    HttpResponse::with_body(StatusCode::OK, asset.body())
}

#[get("/{owner}/{repo}/{file}")]
async fn get_page_orf<'a>(
    path: web::Path<(String, String, String)>,
    data: web::Data<RouteSharedData<'a>>,
) -> impl Responder {
    let (owner, repo, file) = path.into_inner();

    get_page_master(data, owner, repo, None, file).await
}

#[get("/{owner}/{repo}:{branch}/{file}")]
async fn get_page_orbf<'a>(
    path: web::Path<(String, String, String, String)>,
    data: web::Data<RouteSharedData<'a>>,
) -> impl Responder {
    let (owner, repo, branch, file) = path.into_inner();

    let branch = branch.clone();

    get_page_master(data, owner, repo, Some(&branch), file).await
}

#[get("/{owner}/{repo}")]
async fn get_page_or<'a>(
    path: web::Path<(String, String)>,
    data: web::Data<RouteSharedData<'a>>,
) -> impl Responder {
    let (owner, repo) = path.into_inner();

    get_page_master(data, owner, repo, None, "index.html".to_string()).await
}
