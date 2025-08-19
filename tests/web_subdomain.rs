use std::{path::Path, str::FromStr};

use actix_web::{App, http::header::ContentType, middleware::NormalizePath, test};
use pageshelf::{
    asset::{Asset, AssetQueryable},
    conf::ServerConfig,
    page::{PageSource, PageSourceFactory},
    providers::{assets::memory::MemoryAsset, testing::create_example_provider_factory},
    routes::setup_service_config,
};
use url::{Host, Url};

#[tokio::test]
async fn page_subdomain_default_user() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let path = Path::new("/index.html");
    let path_long = Path::new("/my/long/path/index.html");
    let asset = MemoryAsset::from_str("meow");

    let mut config = ServerConfig::default();
    config.pages_urls = Some(vec![Url::from_str("https://example.domain").unwrap()]);
    let factory = create_example_provider_factory()
        .with_asset("owner_1", "pages", "pages", &path, asset.clone())
        .with_asset("owner_2", "other_thing", "pages", &path, asset.clone())
        .with_asset("owner_1", "pages", "pages", &path_long, asset.clone());

    let app = test::init_service(App::new().configure(move |f| {
        setup_service_config(f, &config, factory, None);
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
    assert_eq!(body, asset.body());

    let req = test::TestRequest::get()
        .uri("/index.html")
        .insert_header(("Host", "owner_1.example.domain"))
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let body = test::read_body(resp).await;
    assert_eq!(body, asset.body());

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
    assert_eq!(body, asset.body());

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
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let path = Path::new("/index.html");
    let path_long = Path::new("/my/long/path/index.html");
    let asset_1 = MemoryAsset::from_str("meow");
    let asset_2 = MemoryAsset::from_str("meow");

    let mut config = ServerConfig::default();
    config.pages_urls = Some(vec![Url::from_str("https://example.domain").unwrap()]);
    let factory = create_example_provider_factory()
        .with_asset("owner_1", "pages", "pages", &path, asset_1.clone())
        .with_asset("owner_2", "other_thing", "pages", &path, asset_2.clone())
        .with_asset(
            "owner_2",
            "other_thing",
            "pages",
            &path_long,
            asset_2.clone(),
        );

    let app = test::init_service(App::new().configure(move |f| {
        setup_service_config(f, &config, factory, None);
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
    assert_eq!(body, asset_1.body());

    // Owner 2 has no default page, should fail
    let req = test::TestRequest::get()
        .uri("/")
        .insert_header(("Host", "other_thing.owner_2.example.domain"))
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let body = test::read_body(resp).await;
    assert_eq!(body, asset_2.body());

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
        .uri("/")
        .insert_header((
            "Host",
            "other_thing.owner_2.example.domain/my/long/path/index.html",
        ))
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);
    let body = test::read_body(resp).await;
    assert_eq!(body, asset_2.body());

    let req = test::TestRequest::get()
        .uri("/")
        .insert_header((
            "Host",
            "pages.owner_1.example.domain/my/long/path/index.html",
        ))
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);
}

#[tokio::test]
async fn page_base_priority() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let path = Path::new("/index.html");
    let path_long = Path::new("/my/long/path/index.html");
    let asset = MemoryAsset::from_str("meow");

    let mut config = ServerConfig::default();
    config.default_user = "user".to_string();
    config.url = Some(Url::from_str("https://example.domain").unwrap());
    config.pages_urls = Some(vec![Url::from_str("https://example.domain").unwrap()]);
    let factory = create_example_provider_factory()
        .with_asset("user", "pages", "pages", &path, asset.clone())
        .with_asset("user", "pages", "pages", &path_long, asset.clone());

    let app = test::init_service(App::new().configure(move |f| {
        setup_service_config(f, &config, factory, None);
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
    assert_ne!(body, asset.body());

    // If we specify a page, however, the page will take priority
    let req = test::TestRequest::get()
        .uri("/")
        .insert_header(("Host", "user.example.domain"))
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let body = test::read_body(resp).await;
    assert_eq!(body, asset.body());
}
