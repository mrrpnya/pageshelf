use color_eyre::eyre;
use pageshelf_test_utils::forgejo::ForgejoContainer;
use reqwest::Client;
use rstest::rstest;

// This test is ignored by default because it requires Docker. Run with:
// RUSTFLAGS='--cfg tokio_unstable' cargo test -p pageshelf-test-utils --test forgejo_integration -- --ignored

#[rstest]
#[tokio::test]
#[ignore]
async fn add_and_fetch_asset() {
    let container = ForgejoContainer::with_tag("13")
        .await
        .expect("start container");
    let owner = "testuser";
    let project = "testrepo";
    let channel = "main";
    let path = "assets/foo.txt";
    let content = b"hello-from-test";

    container
        .add_asset(owner, project, channel, path, content)
        .await;

    let url = format!(
        "http://localhost:{}/repo/{}/{}/raw/{}/{}",
        container.port(),
        owner,
        project,
        channel,
        path
    );
    let client = Client::new();
    let resp = client.get(&url).send().await.expect("request");
    assert!(resp.status().is_success());
    let body = resp.bytes().await.expect("read");
    assert_eq!(&body[..], &content[..]);
}
