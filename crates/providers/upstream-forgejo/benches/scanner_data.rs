use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use pageshelf_provider_upstream_forgejo::scanner::data::{
    ChannelData, OwnerData, ProjectData, RepoMap,
};
use rand::{rng, seq::SliceRandom};
use std::sync::Arc;

fn repo_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("RepoMap");

    let owner: Arc<str> = Arc::from("alice");
    let project: Arc<str> = Arc::from("proj1");

    // Test different repo sizes
    for &size in &[100, 1_000, 10_000] {
        let mut repo = RepoMap::default();
        repo.insert_owner(owner.clone(), OwnerData {});
        repo.insert_project(&owner, project.clone(), ProjectData {});

        // Pre-insert channels
        for i in 0..size {
            let channel = Arc::from(format!("channel{}", i));
            let data = ChannelData {
                version: format!("v{}", i),
                valid_assets: Default::default(),
            };
            repo.insert_channel(&owner, &project, channel, data);
        }

        // Prepare randomized channel names
        let mut channels: Vec<String> = (0..size).map(|i| format!("channel{}", i)).collect();
        let mut rng = rng();
        channels.shuffle(&mut rng);

        // Lookup benchmark
        group.bench_with_input(BenchmarkId::new("get_channel", size), &repo, |b, repo| {
            b.iter(|| {
                for ch in &channels {
                    std::hint::black_box(repo.get_channel(&owner, &project, ch));
                }
            });
        });

        // Insertion benchmark (new channels)
        group.bench_with_input(
            BenchmarkId::new("insert_channel", size),
            &repo,
            |b, repo| {
                b.iter(|| {
                    let mut repo = repo.clone();
                    for i in size..size + 100 {
                        let channel = Arc::from(format!("channel{}", i));
                        let data = ChannelData {
                            version: format!("v{}", i),
                            valid_assets: Default::default(),
                        };
                        std::hint::black_box(repo.insert_channel(&owner, &project, channel, data));
                    }
                });
            },
        );

        // Deletion benchmark
        group.bench_with_input(
            BenchmarkId::new("delete_channel", size),
            &repo,
            |b, repo| {
                b.iter(|| {
                    let mut repo = repo.clone();
                    for ch in channels.iter().take(100) {
                        std::hint::black_box(repo.delete_channel(
                            &owner,
                            &project,
                            &Arc::from(ch.clone()),
                        ));
                    }
                });
            },
        );

        // Retain benchmark
        group.bench_with_input(
            BenchmarkId::new("retain_channels_even", size),
            &repo,
            |b, repo| {
                b.iter(|| {
                    let mut repo = repo.clone();
                    repo.retain_channels(|_, _, channel, _| {
                        let num: usize = channel[7..].parse().unwrap_or(0);
                        num.is_multiple_of(2)
                    });
                });
            },
        );

        // Iteration benchmarks
        group.bench_with_input(
            BenchmarkId::new("iterate_owners", size),
            &repo,
            |b, repo| {
                b.iter(|| {
                    for owner in repo.owners() {
                        std::hint::black_box(owner);
                    }
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("iterate_projects_for_owner", size),
            &repo,
            |b, repo| {
                b.iter(|| {
                    for (_, project_data) in repo.projects_for_owner(&owner) {
                        std::hint::black_box(project_data);
                    }
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("iterate_channels_for_project", size),
            &repo,
            |b, repo| {
                b.iter(|| {
                    for (_, channel_data) in repo.channels_for_project(&owner, &project) {
                        std::hint::black_box(channel_data);
                    }
                });
            },
        );

        // Clone benchmark
        group.bench_with_input(BenchmarkId::new("clone_repo", size), &repo, |b, repo| {
            b.iter(|| {
                std::hint::black_box(repo.clone());
            });
        });
    }

    group.finish();
}

criterion_group!(benches, repo_benchmark);
criterion_main!(benches);
