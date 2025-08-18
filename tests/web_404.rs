use std::path::Path;

use actix_web::{App, http::header::ContentType, middleware::NormalizePath, test};
use pageshelf::{
    asset::{Asset, AssetQueryable},
    conf::ServerConfig,
    page::{PageSource, PageSourceFactory},
    providers::{assets::memory::MemoryAsset, testing::create_example_provider_factory},
    routes::setup_service_config,
};

#[tokio::test]
async fn page_server_404() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let factory = create_example_provider_factory();

    let config = ServerConfig::default();

    let app = test::init_service(App::new().configure(move |f| {
        setup_service_config(f, &config, factory, None);
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
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let path_1 = Path::new("/404.html");
    let path_2 = Path::new("/other.html");

    let asset_1 = MemoryAsset::from_str("meow");

    let config = ServerConfig::default();
    let factory = create_example_provider_factory()
        .with_asset("owner_1", "name_1", "pages", &path_2, asset_1.clone())
        .with_asset("owner_1", "name_1", "with_404", &path_1, asset_1.clone());

    let provider = factory.build().unwrap();
    assert!(provider.page_at("owner_1", "name_1", "pages").await.is_ok());
    assert!(
        provider
            .page_at("owner_1", "name_1", "with_404")
            .await
            .unwrap()
            .asset_at(&path_1)
            .await
            .is_ok()
    );

    let app = test::init_service(App::new().wrap(NormalizePath::trim()).configure(move |f| {
        setup_service_config(f, &config, factory, None);
    }))
    .await;

    let req = test::TestRequest::get()
        .uri("/owner_1/name_1")
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);
    let body = test::read_body(resp).await;
    assert_ne!(body, asset_1.body());

    let req = test::TestRequest::get()
        .uri("/owner_1/name_1/index.html")
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);
    let body = test::read_body(resp).await;
    assert_ne!(body, asset_1.body());

    let req = test::TestRequest::get()
        .uri("/owner_1/name_1:with_404")
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);
    let body = test::read_body(resp).await;
    assert_eq!(body, asset_1.body());

    let req = test::TestRequest::get()
        .uri("/owner_1/name_1:with_404/index.html")
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);
    let body = test::read_body(resp).await;
    assert_eq!(body, asset_1.body());
}
