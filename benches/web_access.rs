use std::{path::Path, str::FromStr, sync::Arc};

use actix_web::{App, http::header::ContentType, test};
use criterion::{Criterion, async_executor::AsyncStdExecutor, criterion_group, criterion_main};
use pageshelf::{
    PageSourceFactory,
    conf::ServerConfig,
    frontend::setup_service_config,
    provider::{memory::MemoryAsset, testing::create_example_provider_factory},
};
use url::Url;

fn bench_access_index(c: &mut Criterion) {
    let factory = create_example_provider_factory();

    let config = ServerConfig::default();

    let func = async || {
        let app = test::init_service(App::new().configure(move |f| {
            let provider = Arc::new(factory.build().unwrap());
            setup_service_config(f, &config, provider, None);
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
            let provider = Arc::new(factory.build().unwrap());
            setup_service_config(f, &config, provider, None);
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

criterion_group!(basic_access, bench_access_index, bench_access_page_index);
criterion_main!(basic_access);
