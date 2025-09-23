use std::{path::Path, str::FromStr, sync::Arc};

use actix_web::{App, http::header::ContentType, test};
use pageshelf::{
    Asset, PageSourceFactory,
    conf::ServerConfig,
    frontend::setup_service_config,
    provider::{memory::MemoryAsset, testing::create_example_provider_factory},
};
use url::Url;

#[tokio::test]
async fn page_domain_custom() {
    let mut config = ServerConfig {
        allow_domains: true,
        ..ServerConfig::default()
    };
    config.pages_urls = Some(vec![Url::from_str("https://example.domain").unwrap()]);
    exec_domain_custom(&config).await;
    config.url = Some(Url::from_str("https://root.domain").unwrap());
    exec_domain_custom(&config).await;
}

async fn exec_domain_custom(config: &ServerConfig) {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let path_domains = Path::new("/.domain");
    let path_index = Path::new("/index.html");
    let path_other = Path::new("/other.html");
    let path_long = Path::new("/my/long/path/index.html");
    let asset_domains = MemoryAsset::from("example_custom.domain\nwww.example_custom.domain");
    let asset_index = MemoryAsset::from("meow");
    let asset_other = MemoryAsset::from("meow");

    let factory = create_example_provider_factory()
        .with_asset(
            "owner_1",
            "pages",
            "pages",
            path_domains,
            asset_domains.clone(),
        )
        .with_asset("owner_1", "pages", "pages", path_index, asset_index.clone())
        .with_asset("owner_1", "pages", "pages", path_other, asset_other.clone())
        .with_asset("owner_1", "pages", "pages", path_long, asset_other.clone())
        .with_asset(
            "owner_2",
            "other_thing",
            "pages",
            path_index,
            asset_index.clone(),
        )
        .with_asset(
            "owner_2",
            "other_thing",
            "pages",
            path_long,
            asset_index.clone(),
        );

    let app = test::init_service(App::new().configure(move |f| {
        let provider = Arc::new(factory.build());
        setup_service_config(f, config, provider, config.url_resolver(), None);
    }))
    .await;

    // Owner 1 has a domain page, should succeed
    let req = test::TestRequest::get()
        .uri("/")
        .insert_header(("Host", "example_custom.domain"))
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let body = test::read_body(resp).await;
    assert_eq!(body, asset_index.body().unwrap());

    let req = test::TestRequest::get()
        .uri("/")
        .insert_header(("Host", "www.example_custom.domain"))
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let body = test::read_body(resp).await;
    assert_eq!(body, asset_index.body().unwrap());

    let req = test::TestRequest::get()
        .uri("/index.html")
        .insert_header(("Host", "example_custom.domain"))
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let body = test::read_body(resp).await;
    assert_eq!(body, asset_index.body().unwrap());

    let req = test::TestRequest::get()
        .uri("/other.html")
        .insert_header(("Host", "example_custom.domain"))
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let body = test::read_body(resp).await;
    assert_eq!(body, asset_other.body().unwrap());

    let req = test::TestRequest::get()
        .uri("/")
        .insert_header(("Host", "invalid_domain.domain"))
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);

    /* ---------------------------- Long path testing --------------------------- */

    let req = test::TestRequest::get()
        .uri("/my/long/path/index.html")
        .insert_header(("Host", "example_custom.domain"))
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);
    let body = test::read_body(resp).await;
    assert_eq!(body, asset_other.body().unwrap());

    let req = test::TestRequest::get()
        .uri("/my/long/path/index.html")
        .insert_header(("Host", "invalid_domain.domain"))
        .insert_header(ContentType::plaintext())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);
}
