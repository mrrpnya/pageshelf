use std::{net::TcpListener, path::Path, sync::Arc};

use pageshelf_core::{
    event::EventBus, resolution::default_page_resolver, upstream::mock::MockUpstream,
};
use pageshelf_frontend::renderer::jinja::JinjaRenderer;
use pageshelf_frontend::{DefaultFrontend, Frontend};
use pageshelf_test_utils::setup_test_logger;
use pageshelf_web::actix::ActixWebServer;

use rstest::rstest;

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
#[case("/nope", 404)]
async fn server_404_integration(#[case] path: &str, #[case] expected_status: u16) {
    setup_test_logger();

    let provider = Arc::new(MockUpstream::default());
    let frontend = Arc::new(DefaultFrontend::new(
        JinjaRenderer::default(),
        default_page_resolver(provider.clone(), vec![], EventBus::default()).await,
        provider,
    ));

    let (port, _server_handle) = start_server(frontend).await;

    let url = format!("http://127.0.0.1:{}{}", port, path);
    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await.unwrap();
    assert_eq!(resp.status().as_u16(), expected_status);
}

#[rstest]
#[tokio::test]
#[case("/owner_1/name_1", false)]
#[case("/owner_1/name_1/index.html", false)]
#[case("/owner_1/name_1:with_404", true)]
#[case("/owner_1/name_1:with_404/index.html", true)]
async fn custom_page_404_integration(#[case] uri: &str, #[case] expect_body: bool) {
    setup_test_logger();

    let asset = b"meow";

    let provider = Arc::new(
        MockUpstream::default()
            .with_asset("owner_1", "name_1", "pages", Path::new("other.html"), asset)
            .with_asset(
                "owner_1",
                "name_1",
                "with_404",
                Path::new("404.html"),
                asset,
            ),
    );

    let frontend = Arc::new(DefaultFrontend::new(
        JinjaRenderer::default(),
        default_page_resolver(provider.clone(), vec![], EventBus::default()).await,
        provider,
    ));

    let (port, _server_handle) = start_server(frontend).await;

    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}{}", port, uri);
    let resp = client.get(&url).send().await.unwrap();
    assert_eq!(resp.status().as_u16(), 404);

    let body = resp.bytes().await.unwrap();
    if expect_body {
        assert_eq!(&*body, asset);
    } else {
        assert_ne!(&*body, asset);
    }
}
