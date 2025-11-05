use std::{path::Path, sync::Arc};

use actix_web::{App, http::header::ContentType, middleware::NormalizePath, test};
use pageshelf::ext::Normalizable;
use pageshelf::{
    Asset, AssetSource,
    conf::ServerConfig,
    frontend::{DefaultFrontend, renderer::jinja::JinjaFrontendRenderer},
    log::setup_logger,
    project::source::ProjectSource,
    provider::memory::{MemoryAsset, testing::create_example_provider},
    server::actix::setup_service_config,
};
use tracing::info;

#[tokio::test]
async fn page_server_404() {
    setup_logger(tracing::Level::DEBUG, true);

    let factory = create_example_provider();

    let config = ServerConfig::default();
    let resolver = config.url_resolver();

    let app = test::init_service(App::new().configure(move |f| {
        let provider = factory;
        setup_service_config(
            f,
            Arc::new(DefaultFrontend::new(
                JinjaFrontendRenderer::default(),
                resolver,
                provider,
            )),
        );
    }))
    .await;

    let req = test::TestRequest::get()
        .uri("/")
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

/// Verify that custom 404s work
#[tokio::test]
async fn page_custom_404() {
    setup_logger(tracing::Level::DEBUG, true);

    let path_1 = Path::new("./404.html");

    info!("test 404 path: {:?}", path_1.normalized());
    let path_2 = Path::new("./other.html");

    let asset_1 = MemoryAsset::from("meow");

    let config = ServerConfig::default();
    let resolver = config.url_resolver();
    let provider = create_example_provider()
        .with_asset("owner_1", "name_1", "pages", path_2, asset_1.clone())
        .with_asset("owner_1", "name_1", "with_404", path_1, asset_1.clone());

    let app = test::init_service(App::new().wrap(NormalizePath::trim()).configure(move |f| {
        setup_service_config(
            f,
            Arc::new(DefaultFrontend::new(
                JinjaFrontendRenderer::default(),
                resolver,
                provider,
            )),
        );
    }))
    .await;

    let req = test::TestRequest::get()
        .uri("/owner_1/name_1")
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);
    let body = test::read_body(resp).await;
    assert_ne!(body, asset_1.body().unwrap());

    let req = test::TestRequest::get()
        .uri("/owner_1/name_1/index.html")
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);
    let body = test::read_body(resp).await;
    assert_ne!(body, asset_1.body().unwrap());

    let req = test::TestRequest::get()
        .uri("/owner_1/name_1:with_404")
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);
    let body = test::read_body(resp).await;
    assert_eq!(body, asset_1.body().unwrap());

    let req = test::TestRequest::get()
        .uri("/owner_1/name_1:with_404/index.html")
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);
    let body = test::read_body(resp).await;
    assert_eq!(body, asset_1.body().unwrap());
}
