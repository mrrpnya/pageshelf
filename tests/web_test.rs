use std::{path::Path, sync::Arc};

use actix_web::{App, http::header::ContentType, middleware::NormalizePath, test};
use pageshelf::{
    AssetSource,
    conf::ServerConfig,
    frontend::setup_service_config,
    provider::{memory::MemoryAsset, testing::create_example_provider_factory},
    {PageSource, PageSourceFactory},
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
        let provider = Arc::new(factory.build());
        setup_service_config(f, &config, provider, config.url_resolver(), None);
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
        let provider = Arc::new(factory.build());
        setup_service_config(f, &config, provider, config.url_resolver(), None);
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
    let asset = MemoryAsset::from("meow");

    let config = ServerConfig::default();
    let factory =
        create_example_provider_factory().with_asset("owner_1", "name_1", "pages", path, asset);

    let provider = factory.build();
    assert!(
        provider
            .page_at(
                "owner_1".to_string(),
                "name_1".to_string(),
                "pages".to_string()
            )
            .await
            .is_ok()
    );

    let app = test::init_service(App::new().wrap(NormalizePath::trim()).configure(move |f| {
        let provider = Arc::new(factory.build());
        setup_service_config(f, &config, provider, config.url_resolver(), None);
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
        let provider = Arc::new(factory.build());
        setup_service_config(f, &config, provider, config.url_resolver(), None);
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
    let asset_1 = MemoryAsset::from("meow");
    let asset_2 = MemoryAsset::from("nya");

    let config = ServerConfig::default();
    let factory = create_example_provider_factory()
        .with_asset("owner_1", "name_1", "pages", path, asset_1)
        .with_asset("owner_1", "name_1", "second", path, asset_2);

    let provider = factory.build();
    assert!(
        provider
            .page_at(
                "owner_1".to_string(),
                "name_1".to_string(),
                "pages".to_string()
            )
            .await
            .is_ok()
    );
    assert!(
        provider
            .page_at(
                "owner_1".to_string(),
                "name_1".to_string(),
                "second".to_string()
            )
            .await
            .is_ok()
    );
    assert!(
        provider
            .page_at(
                "owner_1".to_string(),
                "name_1".to_string(),
                "second".to_string()
            )
            .await
            .unwrap()
            .get_asset(path)
            .await
            .is_ok()
    );

    let app = test::init_service(App::new().wrap(NormalizePath::trim()).configure(move |f| {
        let provider = Arc::new(factory.build());
        setup_service_config(f, &config, provider, config.url_resolver(), None);
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
