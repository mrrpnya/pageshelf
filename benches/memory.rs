use std::path::Path;

use criterion::{Criterion, async_executor::AsyncStdExecutor, criterion_group, criterion_main};
use pageshelf::{
    asset::{Asset, AssetQueryable},
    backend::{MemoryPageProviderFactory, memory::MemoryAsset},
    page::{PageSource, PageSourceFactory},
};
use rand::{Rng, distr::Alphanumeric};

pub fn one_page_one_file(c: &mut Criterion) {
    let owner = "spamton";
    let name = "shop";
    let branch = "unstable";
    let path = Path::new("/neo");

    let asset = MemoryAsset::from_str("Big shot");

    let provider = MemoryPageProviderFactory::new()
        .with_asset(owner, name, branch, path, asset.clone())
        .build()
        .unwrap();

    let func = async || {
        let p = provider
            .page_at(owner.to_string(), name.to_string(), branch.to_string())
            .await
            .unwrap();
        let a = p.asset_at(path).await.unwrap();
        let b = asset.body();
        assert!(a.body() == b);
    };

    c.bench_function("Memory Page/Asset: One Page, One File", |b| {
        b.to_async(AsyncStdExecutor).iter(|| func())
    });
}

pub fn one_page_many_file(c: &mut Criterion) {
    let owner = "spamton";
    let name = "shop";
    let branch = "unstable";
    let path = Path::new("/neo");

    let asset = MemoryAsset::from_str("Big shot");
    let asset_other = MemoryAsset::from_str("TV Time");

    let mut factory =
        MemoryPageProviderFactory::new().with_asset(owner, name, branch, path, asset.clone());

    for _ in 0..2048 {
        let file: String = rand::rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect();
        let file = Path::new(&file);

        factory = factory.with_asset(owner, name, branch, file, asset_other.clone())
    }

    let provider = factory.build().unwrap();

    let func = async || {
        let p = provider
            .page_at(owner.to_string(), name.to_string(), branch.to_string())
            .await
            .unwrap();
        let a = p.asset_at(path).await.unwrap();
        let b = asset.body();
        assert!(a.body() == b);
    };

    c.bench_function("Memory Page/Asset: One Page, Many Files", |b| {
        b.to_async(AsyncStdExecutor).iter(|| func())
    });
}

criterion_group!(memory, one_page_one_file, one_page_many_file);
criterion_main!(memory);
