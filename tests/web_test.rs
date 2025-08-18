use std::{path::Path, str::FromStr};

use actix_web::{App, http::header::ContentType, middleware::NormalizePath, test};
use pageshelf::{
    asset::AssetQueryable,
    conf::ServerConfig,
    page::{PageSource, PageSourceFactory},
    providers::{assets::memory::MemoryAsset, testing::create_example_provider_factory},
    routes::setup_service_config,
};

/* -------------------------------------------------------------------------- */
/*                            Server Page accessing                           */
/* -------------------------------------------------------------------------- */

#[tokio::test]
async fn page_server_index() {
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

/* -------------------------------------------------------------------------- */
/*                               Page Accessing                               */
/* -------------------------------------------------------------------------- */

/// Ensure pages are accessible with owner-name-asset
#[tokio::test]
async fn page_access_owner_name_asset() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let config = ServerConfig::default();
    let factory = create_example_provider_factory();

    let app = test::init_service(App::new().wrap(NormalizePath::trim()).configure(move |f| {
        setup_service_config(f, &config, factory, None);
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
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let path = Path::new("/index.html");
    let asset = MemoryAsset::from_str("meow");

    let config = ServerConfig::default();
    let factory =
        create_example_provider_factory().with_asset("owner_1", "name_1", "pages", &path, asset);

    let provider = factory.build().unwrap();
    assert!(provider.page_at("owner_1", "name_1", "pages").await.is_ok());

    let app = test::init_service(App::new().wrap(NormalizePath::trim()).configure(move |f| {
        setup_service_config(f, &config, factory, None);
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
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let config = ServerConfig::default();
    let factory = create_example_provider_factory();

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

    let req = test::TestRequest::get()
        .uri("/owner_2/name_1")
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);
}

/// Ensure that specific branches of a Page are accessible
#[tokio::test]
async fn page_access_branch() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let path = Path::new("/index.html");
    let asset_1 = MemoryAsset::from_str("meow");
    let asset_2 = MemoryAsset::from_str("mrrp");

    let config = ServerConfig::default();
    let factory = create_example_provider_factory()
        .with_asset("owner_1", "name_1", "pages", &path, asset_1)
        .with_asset("owner_1", "name_1", "second", &path, asset_2);

    let provider = factory.build().unwrap();
    assert!(provider.page_at("owner_1", "name_1", "pages").await.is_ok());
    assert!(
        provider
            .page_at("owner_1", "name_1", "second")
            .await
            .is_ok()
    );
    assert!(
        provider
            .page_at("owner_1", "name_1", "second")
            .await
            .unwrap()
            .asset_at(&path)
            .await
            .is_ok()
    );

    let app = test::init_service(App::new().wrap(NormalizePath::trim()).configure(move |f| {
        setup_service_config(f, &config, factory, None);
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
