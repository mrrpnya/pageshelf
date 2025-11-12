use std::{path::Path, sync::Arc};

use criterion::{Criterion, criterion_group, criterion_main};
use pageshelf_core::upstream::{Upstream, mock::MockUpstream, source::AssetSource};
use rand::{Rng, distr::Alphanumeric, seq::IndexedRandom};
use tokio::runtime::Runtime;

const OWNER: &str = "spamton";
const PROJECT: &str = "shop";
const CHANNEL: &str = "unstable";

/// Generates a random alphanumeric string of the specified length.
fn random_string(len: usize) -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

/// Benchmark: Single project, single asset.
/// Measures the time to fetch one asset from memory.
pub fn one_page_one_file(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let path = Path::new("/neo");

    let asset = b"Big shot";
    let provider = MockUpstream::default().with_asset(OWNER, PROJECT, CHANNEL, path, asset);

    let func = async || {
        let a = provider
            .get_asset_bytes(OWNER, PROJECT, CHANNEL, path)
            .await
            .unwrap();
        assert!(&*a == asset);
    };

    c.bench_function("Memory Page/Asset: One Page, One File", |b| {
        b.to_async(&rt).iter(func)
    });
}

/// Benchmark: Single project with many assets (2048 files).
/// Measures performance when fetching a single asset from a large collection.
pub fn one_page_many_file(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let path = Path::new("/neo");

    let asset = b"Big shot";
    let asset_other = b"TV Time";

    let mut provider = MockUpstream::default().with_asset(OWNER, PROJECT, CHANNEL, path, asset);

    for _ in 0..2048 {
        let r = random_string(7);
        let file_path = Path::new(&r);
        provider = provider.with_asset(OWNER, PROJECT, CHANNEL, file_path, asset_other);
    }

    let func = async || {
        let a = provider
            .get_asset_bytes(OWNER, PROJECT, CHANNEL, path)
            .await
            .unwrap();
        assert!(&*a == asset);
    };

    c.bench_function("Memory Page/Asset: One Page, Many Files", |b| {
        b.to_async(&rt).iter(func)
    });
}

/// Benchmark: Many projects with many assets per project (16 projects, 512 files each).
/// Includes small and large assets to measure memory access patterns at scale.
pub fn many_pages_many_files(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let asset_small = b"Short text";
    let asset_large = b"A".repeat(1024); // 1 KB asset

    let mut provider = MockUpstream::default();

    for page_index in 0..16 {
        let project_name = format!("project_{page_index}");
        let path = Path::new("/index.html");
        provider = provider.with_asset(OWNER, &project_name, CHANNEL, path, asset_small);

        // Add many files per project
        for _ in 0..512 {
            let r = random_string(10);
            let file_path = Path::new(&r);
            provider = provider.with_asset(
                OWNER,
                &project_name,
                CHANNEL,
                file_path,
                asset_large.as_slice(),
            );
        }
    }

    let func = async || {
        for page_index in 0..16 {
            let project_name = format!("project_{page_index}");
            let a = provider
                .get_asset_bytes(OWNER, &project_name, CHANNEL, Path::new("/index.html"))
                .await
                .unwrap();
            assert!(&*a == asset_small);
        }
    };

    c.bench_function("Memory Page/Asset: Many Pages, Many Files", |b| {
        b.to_async(&rt).iter(func)
    });
}

/// Benchmark: Parallel fetching of assets from a single project.
/// Measures async fetch performance under concurrent workload.
pub fn parallel_asset_fetch_variants(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    const TOTAL_ASSETS: usize = 1024; // Total number of assets to generate
    const BATCH_SIZE: usize = 64; // Batch size for batched fetch
    const SUBSET_SIZE: usize = TOTAL_ASSETS / 10; // 10% of assets for random subset

    let asset = b"Big shot";
    let mut provider = MockUpstream::default();

    // Pre-generate asset paths
    let asset_paths: Vec<String> = (0..TOTAL_ASSETS).map(|_| random_string(8)).collect();
    for path_str in &asset_paths {
        let path = Path::new(path_str);
        provider = provider.with_asset(OWNER, PROJECT, CHANNEL, path, asset);
    }

    // Wrap provider in Arc so we can share it across concurrent tasks for the
    // parallel fetch benchmarks. The mock's internal structures are Arc-backed
    // so reads are cheap.
    let provider = Arc::new(provider);

    // --- 1. Full parallel fetch ---
    c.bench_function("Parallel Fetch: Full Assets", |b| {
        let provider = Arc::clone(&provider);
        let asset_paths = asset_paths.clone();
        let rt = Runtime::new().unwrap();
        b.to_async(rt).iter(|| {
            let provider = Arc::clone(&provider);
            let asset_paths = asset_paths.clone();
            async move {
                let handles = asset_paths
                    .iter()
                    .map(|p_str| {
                        let path_string = p_str.clone();
                        let provider_ref = Arc::clone(&provider);
                        tokio::spawn(async move {
                            let path = Path::new(&path_string);
                            let _ = provider_ref
                                .get_asset_bytes(OWNER, PROJECT, CHANNEL, path)
                                .await;
                        })
                    })
                    .collect::<Vec<_>>();

                futures::future::join_all(handles).await;
            }
        })
    });

    // --- 2. Sequential fetch ---
    c.bench_function("Parallel Fetch: Sequential Access", |b| {
        let provider = Arc::clone(&provider);
        let asset_paths = asset_paths.clone();
        b.to_async(&rt).iter(move || {
            let provider = Arc::clone(&provider);
            let asset_paths = asset_paths.clone();
            async move {
                for path_str in asset_paths {
                    let path = Path::new(&path_str);
                    let _ = provider
                        .get_asset_bytes(OWNER, PROJECT, CHANNEL, path)
                        .await;
                }
            }
        })
    });

    // --- 3. Batched fetch ---
    c.bench_function("Parallel Fetch: Batched Assets", |b| {
        let provider = Arc::clone(&provider);
        let asset_paths = asset_paths.clone();
        b.to_async(&rt).iter(move || {
            let provider = Arc::clone(&provider);
            let asset_paths = asset_paths.clone();
            async move {
                for batch in asset_paths.chunks(BATCH_SIZE) {
                    let handles = batch
                        .iter()
                        .map(|p_str| {
                            let path_string = p_str.clone();
                            let provider_ref = Arc::clone(&provider);
                            tokio::spawn(async move {
                                let path = Path::new(&path_string);
                                let _ = provider_ref
                                    .get_asset_bytes(OWNER, PROJECT, CHANNEL, path)
                                    .await;
                            })
                        })
                        .collect::<Vec<_>>();
                    futures::future::join_all(handles).await;
                }
            }
        })
    });

    // --- 4. Random subset fetch ---
    c.bench_function("Parallel Fetch: Random Subset", |b| {
        let provider = Arc::clone(&provider);
        let asset_paths = asset_paths.clone();
        b.to_async(&rt).iter(move || {
            let provider = Arc::clone(&provider);
            let asset_paths = asset_paths.clone();
            async move {
                let mut rng = rand::rng();
                let subset_paths: Vec<String> = asset_paths
                    .choose_multiple(&mut rng, SUBSET_SIZE)
                    .cloned()
                    .collect();

                let handles = subset_paths
                    .iter()
                    .map(|p_str| {
                        let path_string = p_str.clone();
                        let provider_ref = Arc::clone(&provider);
                        tokio::spawn(async move {
                            let path = Path::new(&path_string);
                            let _ = provider_ref
                                .get_asset_bytes(OWNER, PROJECT, CHANNEL, path)
                                .await;
                        })
                    })
                    .collect::<Vec<_>>();

                futures::future::join_all(handles).await;
            }
        })
    });
}

criterion_group!(
    memory,
    one_page_one_file,
    one_page_many_file,
    many_pages_many_files,
    parallel_asset_fetch_variants
);
criterion_main!(memory);
