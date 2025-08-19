use std::{fmt::Debug, path::Path, str::FromStr};

use actix_web::{
    App,
    dev::{Service, ServiceResponse},
    http::header::ContentType,
    test,
};
use criterion::{Criterion, async_executor::AsyncStdExecutor, criterion_group, criterion_main};
use pageshelf::{
    conf::ServerConfig,
    page::PageSource,
    providers::{assets::memory::MemoryAsset, testing::create_example_provider_factory},
    routes::setup_service_config,
};
use url::Url;

fn bench_access_index(c: &mut Criterion) {
    // let _ = env_logger::builder()
    //     .is_test(true)
    //     .filter_level(log::LevelFilter::Debug)
    //     .try_init();

    let factory = create_example_provider_factory();

    let config = ServerConfig::default();

    let func = async || {
        let app = test::init_service(App::new().configure(move |f| {
            setup_service_config(f, &config, factory, None);
        }))
        .await;

        let req = test::TestRequest::get()
            .uri("/")
            .insert_header(ContentType::plaintext())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success())
    };

    c.bench_function("Access Server Index Page", |b| {
        b.to_async(AsyncStdExecutor).iter(|| func.clone()())
    });
}

fn bench_access_page_index(c: &mut Criterion) {
    //  let _ = env_logger::builder()
    //      .is_test(true)
    //      .filter_level(log::LevelFilter::Debug)
    //      .try_init();

    let path = Path::new("/index.html");
    let path_long = Path::new("/my/long/path/index.html");
    let asset = MemoryAsset::from_str("meow");

    let mut config = ServerConfig::default();
    config.pages_urls = Some(vec![Url::from_str("https://example.domain").unwrap()]);
    let factory = create_example_provider_factory()
        .with_asset("owner_1", "pages", "pages", &path, asset.clone())
        .with_asset("owner_2", "other_thing", "pages", &path, asset.clone())
        .with_asset("owner_2", "other_thing", "pages", &path_long, asset.clone());

    let config = ServerConfig::default();

    let func = async || {
        let app = test::init_service(App::new().configure(move |f| {
            setup_service_config(f, &config, factory, None);
        }))
        .await;

        let req = test::TestRequest::get()
            .uri("/owner_1/pages")
            .insert_header(ContentType::plaintext())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success())
    };

    c.bench_function("Access Page's Index Page", |b| {
        b.to_async(AsyncStdExecutor).iter(|| func.clone()())
    });
}

criterion_group!(benches, bench_access_index, bench_access_page_index);
criterion_main!(benches);
