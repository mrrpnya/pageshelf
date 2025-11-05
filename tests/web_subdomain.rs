use std::{path::Path, str::FromStr, sync::Arc};

use actix_web::{App, http::header::ContentType, middleware::NormalizePath, test};
use pageshelf::{
    Asset,
    conf::ServerConfig,
    frontend::{DefaultFrontend, renderer::jinja::JinjaFrontendRenderer},
    log::setup_logger,
    provider::{memory::MemoryAsset, testing::create_example_provider},
    server::actix::setup_service_config,
};
use url::Url;

#[tokio::test]
async fn page_subdomain_default_user() {
    setup_logger(tracing::Level::DEBUG, true);

    let mut config = ServerConfig {
        pages_domains: Some(vec![Url::from_str("https://example.domain").unwrap()]),
        ..ServerConfig::default()
    };
    exec_subdomain_default_user(&config).await;
    config.domain = Some(Url::from_str("https://root.domain").unwrap());
    exec_subdomain_default_user(&config).await;
}

async fn exec_subdomain_default_user(config: &ServerConfig) {
    let path_index = Path::new("/index.html");
    let path_other = Path::new("/other.html");
    let path_long = Path::new("/my/long/path/index.html");
    let asset_index = MemoryAsset::from("meow");
    let asset_other = MemoryAsset::from("nya");

    let provider = create_example_provider()
        .with_asset("owner_1", "pages", "pages", path_index, asset_index.clone())
        .with_asset("owner_1", "pages", "pages", path_other, asset_other.clone())
        .with_asset(
            "owner_2",
            "other_thing",
            "pages",
            path_index,
            asset_index.clone(),
        )
        .with_asset("owner_1", "pages", "pages", path_long, asset_index.clone());

    let resolver = config.url_resolver();

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

    // Owner 1 has a default page, should succeed
    let req = test::TestRequest::get()
        .uri("/")
        .insert_header(("Host", "owner_1.example.domain"))
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let body = test::read_body(resp).await;
    assert_eq!(body, asset_index.body().unwrap());

    let req = test::TestRequest::get()
        .uri("/index.html")
        .insert_header(("Host", "owner_1.example.domain"))
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let body = test::read_body(resp).await;
    assert_eq!(body, asset_index.body().unwrap());

    let req = test::TestRequest::get()
        .uri("/other.html")
        .insert_header(("Host", "owner_1.example.domain"))
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let body = test::read_body(resp).await;
    assert_eq!(body, asset_other.body().unwrap());

    // Owner 2 has no default page, should fail
    let req = test::TestRequest::get()
        .uri("/")
        .insert_header(("Host", "owner_2.example.domain"))
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);

    /* ---------------------------- Long path testing --------------------------- */

    let req = test::TestRequest::get()
        .uri("/my/long/path/index.html")
        .insert_header(("Host", "owner_1.example.domain"))
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);
    let body = test::read_body(resp).await;
    assert_eq!(body, asset_index.body().unwrap());

    let req = test::TestRequest::get()
        .uri("/my/long/path/index.html")
        .insert_header(("Host", "owner_2.example.domain"))
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);
}

#[tokio::test]
async fn page_subdomain_specific() {
    setup_logger(tracing::Level::DEBUG, true);

    let path = Path::new("/index.html");
    let path_long = Path::new("/my/long/path/index.html");
    let asset_1 = MemoryAsset::from("meow");
    let asset_2 = MemoryAsset::from("meow");

    let config = ServerConfig {
        pages_domains: Some(vec![Url::from_str("https://example.domain").unwrap()]),
        ..ServerConfig::default()
    };
    let provider = create_example_provider()
        .with_asset("owner_1", "pages", "pages", path, asset_1.clone())
        .with_asset("owner_2", "other_thing", "pages", path, asset_2.clone())
        .with_asset(
            "owner_2",
            "other_thing",
            "pages",
            path_long,
            asset_2.clone(),
        );

    let resolver = config.url_resolver();

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

    // Owner 1 has a default page, should succeed
    let req = test::TestRequest::get()
        .uri("/")
        .insert_header(("Host", "pages.owner_1.example.domain"))
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let body = test::read_body(resp).await;
    assert_eq!(body, asset_1.body().unwrap());

    let req = test::TestRequest::get()
        .uri("/")
        .insert_header(("Host", "other_thing.owner_2.example.domain"))
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let body = test::read_body(resp).await;
    assert_eq!(body, asset_2.body().unwrap());

    // Owner 2 has no default page, should fail
    let req = test::TestRequest::get()
        .uri("/")
        .insert_header(("Host", "pages.owner_2.example.domain"))
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);

    /* ---------------------------- Long path testing --------------------------- */

    let req = test::TestRequest::get()
        .uri("/my/long/path/index.html")
        .insert_header(("Host", "other_thing.owner_2.example.domain"))
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);
    let body = test::read_body(resp).await;
    assert_eq!(body, asset_2.body().unwrap());

    let req = test::TestRequest::get()
        .uri("/my/long/path/index.html")
        .insert_header(("Host", "pages.owner_1.example.domain"))
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);
}

#[tokio::test]
async fn page_base_priority() {
    setup_logger(tracing::Level::DEBUG, true);

    let config = ServerConfig {
        default_user: "user".to_string(),
        domain: Some(Url::from_str("https://example.domain").unwrap()),
        pages_domains: Some(vec![Url::from_str("https://example.domain").unwrap()]),
        ..ServerConfig::default()
    };

    exec_base_priority(&config).await;
}

async fn exec_base_priority(config: &ServerConfig) {
    setup_logger(tracing::Level::DEBUG, true);

    let path = Path::new("/index.html");
    let path_long = Path::new("/my/long/path/index.html");
    let asset = MemoryAsset::from("meow");

    let provider = create_example_provider()
        .with_asset("user", "pages", "pages", path, asset.clone())
        .with_asset("user", "pages", "pages", path_long, asset.clone());

    let resolver = config.url_resolver();

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

    // Should respond with the built-in index over a pages index
    let req = test::TestRequest::get()
        .uri("/")
        .insert_header(("Host", "example.domain"))
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let body = test::read_body(resp).await;
    assert_ne!(body, asset.body().unwrap());

    // If we specify a page, however, the page will take priority
    let req = test::TestRequest::get()
        .uri("/")
        .insert_header(("Host", "user.example.domain"))
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let body = test::read_body(resp).await;
    assert_eq!(body, asset.body().unwrap());
}
