use std::{net::TcpListener, path::Path, sync::Arc};

use pageshelf_core::{
    event::EventBus, resolution::default_page_resolver, upstream::mock::MockUpstream,
};
use pageshelf_frontend::renderer::jinja::JinjaRenderer;
use pageshelf_frontend::{DefaultFrontend, Frontend};
use pageshelf_test_utils::setup_test_logger;
use pageshelf_web::actix::ActixWebServer;

use rstest::rstest;

/// Starts a live Actix Web server on a random port
async fn start_server<F: Frontend + Send + Sync + 'static>(
    frontend: Arc<F>,
) -> (u16, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();

    let handle = tokio::spawn({
        let frontend = frontend.clone();
        async move {
            let server = ActixWebServer::new(frontend).unwrap();
            server.run_with_listener(listener).await.unwrap();
        }
    });

    (port, handle)
}

/// ------------------- Integration Tests -------------------

#[rstest]
#[tokio::test]
#[case("/", 200, "owner_1")]
#[case("/index.html", 200, "owner_1")]
#[case("/other.html", 200, "owner_1")]
#[case("/", 404, "owner_2")]
#[case("/my/long/path/index.html", 200, "owner_1")]
#[case("/my/long/path/index.html", 404, "owner_2")]
async fn page_subdomain_default_user_tests(
    #[case] uri: &str,
    #[case] expected_status: u16,
    #[case] owner: &str,
) {
    setup_test_logger();

    let asset_index = b"meow";
    let asset_other = b"nya";

    let provider = Arc::new(
        MockUpstream::default()
            .with_asset(
                "owner_1",
                "pages",
                "pages",
                Path::new("/index.html"),
                asset_index,
            )
            .with_asset(
                "owner_1",
                "pages",
                "pages",
                Path::new("/other.html"),
                asset_other,
            )
            .with_asset(
                "owner_1",
                "pages",
                "pages",
                Path::new("/my/long/path/index.html"),
                asset_index,
            )
            .with_asset(
                "owner_2",
                "other_thing",
                "pages",
                Path::new("/index.html"),
                asset_index,
            ),
    );

    let resolver = default_page_resolver(
        provider.clone(),
        vec!["example.com".to_string()],
        EventBus::default(),
    )
    .await;
    let frontend = Arc::new(DefaultFrontend::new(
        JinjaRenderer::default(),
        resolver,
        provider,
    ));

    let (port, _server_handle) = start_server(frontend).await;
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}{}", port, uri);

    let resp = client
        .get(&url)
        .header("Host", format!("{owner}.example.com"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), expected_status);
}

/// ------------------- Subdomain Specific Tests -------------------

#[rstest]
#[tokio::test]
#[case("/", 200, "pages.owner_1")]
#[case("/", 200, "other_thing.owner_2")]
#[case("/", 404, "pages.owner_2")]
#[case("/my/long/path/index.html", 200, "other_thing.owner_2")]
#[case("/my/long/path/index.html", 404, "pages.owner_1")]
async fn page_subdomain_specific_tests(
    #[case] uri: &str,
    #[case] expected_status: u16,
    #[case] host: &str,
) {
    setup_test_logger();

    let asset_1 = b"meow";
    let asset_2 = b"meow";

    let provider = Arc::new(
        MockUpstream::default()
            .with_asset(
                "owner_1",
                "pages",
                "pages",
                Path::new("/index.html"),
                asset_1,
            )
            .with_asset(
                "owner_2",
                "other_thing",
                "pages",
                Path::new("/index.html"),
                asset_2,
            )
            .with_asset(
                "owner_2",
                "other_thing",
                "pages",
                Path::new("/my/long/path/index.html"),
                asset_2,
            ),
    );

    let resolver = default_page_resolver(
        provider.clone(),
        vec!["example.com".to_string()],
        EventBus::default(),
    )
    .await;
    let frontend = Arc::new(DefaultFrontend::new(
        JinjaRenderer::default(),
        resolver,
        provider,
    ));

    let (port, _server_handle) = start_server(frontend).await;
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}{}", port, uri);

    let host = format!("{}.example.com", host);

    let resp = client.get(&url).header("Host", host).send().await.unwrap();
    assert_eq!(resp.status().as_u16(), expected_status);
}

/// ------------------- Base Priority Tests -------------------

#[rstest]
#[tokio::test]
#[case("/", 200)]
#[case("/index.html", 200)]
async fn page_base_priority_tests(#[case] uri: &str, #[case] expected_status: u16) {
    setup_test_logger();

    let asset = b"meow";

    let provider = Arc::new(
        MockUpstream::default()
            .with_asset("user", "pages", "pages", Path::new("/index.html"), asset)
            .with_asset(
                "user",
                "pages",
                "pages",
                Path::new("/my/long/path/index.html"),
                asset,
            ),
    );

    let resolver = default_page_resolver(
        provider.clone(),
        vec!["example.domain".to_string()],
        EventBus::default(),
    )
    .await;
    let frontend = Arc::new(DefaultFrontend::new(
        JinjaRenderer::default(),
        resolver,
        provider,
    ));

    let (port, _server_handle) = start_server(frontend).await;
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}{}", port, uri);

    let resp = client
        .get(&url)
        .header("Host", "user.example.domain")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), expected_status);
}
