use std::{path::Path, sync::Arc};

use actix_web::{App, http::header::ContentType, middleware::NormalizePath, test};
use pageshelf::{
    AssetSource,
    conf::ServerConfig,
    frontend::{DefaultFrontend, renderer::jinja::JinjaFrontendRenderer},
    log::setup_logger,
    project::source::ProjectSource,
    provider::{memory::MemoryAsset, testing::create_example_provider},
    server::actix::setup_service_config,
};

/* -------------------------------------------------------------------------- */
/*                            Server Page accessing                           */
/* -------------------------------------------------------------------------- */

#[tokio::test]
async fn page_server_index() {
    setup_logger(tracing::Level::DEBUG, true);

    let provider = create_example_provider();

    let config = ServerConfig::default();
    let resolver = config.url_resolver();

    let app = test::init_service(App::new().configure(move |f| {
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

/* -------------------------------------------------------------------------- */
/*                               Page Accessing                               */
/* -------------------------------------------------------------------------- */

/// Ensure pages are accessible with owner-name-asset
#[tokio::test]
async fn page_access_owner_name_asset() {
    setup_logger(tracing::Level::DEBUG, true);

    let config = ServerConfig::default();
    let resolver = config.url_resolver();
    let provider = create_example_provider();

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
        .uri("/owner_1/name_1/asset_1")
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);

    let req = test::TestRequest::get()
        .uri("/owner_2/name_1/asset_1")
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);
}

/// Ensure accessing a Page at index is successful
#[tokio::test]
async fn page_access_index() {
    setup_logger(tracing::Level::DEBUG, true);

    let path = Path::new("/index.html");
    let asset = MemoryAsset::from("meow");

    let config = ServerConfig::default();
    let resolver = config.url_resolver();
    let provider = create_example_provider().with_asset("owner_1", "name_1", "pages", path, asset);

    assert!(
        provider
            .has_page("owner_1", "name_1", "pages")
            .await
            .unwrap()
    );

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
    assert_eq!(resp.status().as_u16(), 200);

    let req = test::TestRequest::get()
        .uri("/owner_2/name_1")
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);
}

/// Ensure accessing a Page at index fails when no index is available
#[tokio::test]
async fn page_access_no_index() {
    setup_logger(tracing::Level::DEBUG, true);

    let config = ServerConfig::default();
    let resolver = config.url_resolver();
    let provider = create_example_provider();

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

    let req = test::TestRequest::get()
        .uri("/owner_2/name_1")
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);

    let config = ServerConfig::default();
    let resolver = config.url_resolver();
}

/// Ensure that specific branches of a Page are accessible
#[tokio::test]
async fn page_access_branch() {
    setup_logger(tracing::Level::DEBUG, true);

    let path = Path::new("/index.html");
    let asset_1 = MemoryAsset::from("meow");
    let asset_2 = MemoryAsset::from("nya");

    let config = ServerConfig::default();
    let resolver = config.url_resolver();
    let provider = create_example_provider()
        .with_asset("owner_1", "name_1", "pages", path, asset_1)
        .with_asset("owner_1", "name_1", "second", path, asset_2);

    assert!(
        provider
            .has_page("owner_1", "name_1", "pages")
            .await
            .unwrap()
    );
    assert!(
        provider
            .has_page("owner_1", "name_1", "second")
            .await
            .unwrap()
    );
    assert!(
        provider
            .has_page("owner_1", "name_1", "second")
            .await
            .unwrap()
    );

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
        .uri("/owner_1/name_1/index.html")
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);

    let req = test::TestRequest::get()
        .uri("/owner_1/name_1:second/index.html")
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);

    let req = test::TestRequest::get()
        .uri("/owner_1/name_1:third/index.html")
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);

    let req = test::TestRequest::get()
        .uri("/owner_1/name_1")
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);

    let req = test::TestRequest::get()
        .uri("/owner_1/name_1:second")
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);

    let req = test::TestRequest::get()
        .uri("/owner_1/name_1:third")
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);
}
