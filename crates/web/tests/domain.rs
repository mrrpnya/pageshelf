use std::{net::TcpListener, path::Path, sync::Arc};

use pageshelf_core::upstream::mock::MockUpstream;
use pageshelf_core::{event::EventBus, resolution::default_page_resolver};
use pageshelf_frontend::DefaultFrontend;
use pageshelf_frontend::renderer::jinja::JinjaRenderer;
use pageshelf_test_utils::setup_test_logger;
use pageshelf_web::actix::ActixWebServer;
use rstest::rstest;

/// Represents a single test case
struct TestCase<'a> {
    owner: &'a str,
    project: &'a str,
    path: &'a str,
    content: &'a [u8],
}

/// Helper to start server with given assets
async fn setup_server<'a>(assets: &[TestCase<'a>]) -> (u16, reqwest::Client) {
    setup_test_logger();

    let mut provider = MockUpstream::default();
    for asset in assets {
        let path = Path::new(asset.path);
        provider = provider.with_asset(asset.owner, asset.project, "pages", path, asset.content);
    }

    let provider = Arc::new(provider);
    let resolver = default_page_resolver(
        provider.clone(),
        vec!["example_custom.domain".to_string()],
        EventBus::default(),
    )
    .await;

    let frontend = Arc::new(DefaultFrontend::new(
        JinjaRenderer::default(),
        resolver,
        provider,
    ));

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();

    tokio::spawn({
        let frontend = frontend.clone();
        async move {
            let server = ActixWebServer::new(frontend).unwrap();
            server.run_with_listener(listener).await.unwrap();
        }
    });

    let client = reqwest::Client::new();
    (port, client)
}

/// Parameterized test driven by RStest
#[rstest]
#[tokio::test]
#[case(
    "owner_1",
    "pages",
    "index.html",
    b"index content",
    "example_custom.domain",
    true
)]
#[case(
    "owner_1",
    "pages",
    "other.html",
    b"other content",
    "example_custom.domain",
    true
)]
#[case(
    "owner_1",
    "pages",
    "my/long/path/index.html",
    b"long content",
    "example_custom.domain",
    true
)]
#[case(
    "owner_1",
    "pages",
    "index.html",
    b"index content",
    "invalid.domain",
    false
)]
async fn page_domain_custom_integration(
    #[case] owner: &str,
    #[case] project: &str,
    #[case] path: &str,
    #[case] content: &[u8],
    #[case] host: &str,
    #[case] expect_success: bool,
) {
    let host = format!("{project}.{owner}.{host}");
    // Create asset list for this single test case
    let assets = vec![
        TestCase {
            owner,
            project,
            path,
            content,
        },
        // Add .domain file for custom domain
        TestCase {
            owner,
            project,
            path: ".domain",
            content: b"example_custom.domain",
        },
    ];

    let (port, client) = setup_server(&assets).await;

    let url = format!("http://127.0.0.1:{}/{}", port, path);
    let resp = client
        .get(&url)
        .header("Host", &host)
        .header("Content-Type", "text/plain")
        .send()
        .await
        .unwrap();

    if expect_success {
        assert!(
            resp.status().is_success(),
            "Failed for host: {} path: {}",
            host,
            path
        );
        let body = resp.bytes().await.unwrap();
        assert_eq!(body, content);
    } else {
        assert_eq!(
            resp.status().as_u16(),
            404,
            "Expected 404 for host: {} path: {}",
            host,
            path
        );
    }
}
