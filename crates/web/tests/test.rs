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

#[rstest]
#[tokio::test]
#[case("/", 200)]
#[case("/owner_1/name_1/asset_1", 200)]
#[case("/owner_2/name_1/asset_1", 404)]
#[case("/owner_1/name_1", 200)]
#[case("/owner_2/name_1", 404)]
#[case("/owner_1/name_1/index.html", 200)]
#[case("/owner_1/name_1:second/index.html", 200)]
#[case("/owner_1/name_1:third/index.html", 404)]
async fn integration_tests(#[case] uri: &str, #[case] expected_status: u16) {
    setup_test_logger();

    let asset_1 = b"meow";
    let asset_2 = b"nya";

    let provider = Arc::new(
        MockUpstream::default()
            .with_asset(
                "owner_1",
                "name_1",
                "pages",
                Path::new("/index.html"),
                asset_1,
            )
            .with_asset(
                "owner_1",
                "name_1",
                "second",
                Path::new("/index.html"),
                asset_2,
            )
            .with_asset(
                "owner_1",
                "name_1",
                "asset_1",
                Path::new("/index.html"),
                asset_1,
            )
            .with_asset(
                "owner_1",
                "name_1",
                "with_404",
                Path::new("404.html"),
                asset_1,
            ),
    );

    let resolver = default_page_resolver(provider.clone(), vec![], EventBus::default()).await;
    let frontend = Arc::new(DefaultFrontend::new(
        JinjaRenderer::default(),
        resolver,
        provider,
    ));

    let (port, _server_handle) = start_server(frontend).await;
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}{}", port, uri);

    let resp = client.get(&url).send().await.unwrap();
    assert_eq!(resp.status().as_u16(), expected_status);
}
