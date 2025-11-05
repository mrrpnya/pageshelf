pub mod actix;

pub trait PageshelfWebServer {
    async fn run(&self, host: &str, port: u16);
}
