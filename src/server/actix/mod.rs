mod response;
pub mod routes;
use crate::{frontend::Frontend, server::PageshelfWebServer};
use actix_web::web::{self, ServiceConfig};
use routes::{RoutingState, register_routes_to_config};
use std::sync::Arc;

#[derive(Default)]
pub struct ActixPageshelfWebServer<F: Frontend + Send + Sync + 'static> {
    frontend: Arc<F>,
}

impl<F: Frontend + Send + Sync + 'static> ActixPageshelfWebServer<F> {
    pub fn new(frontend: Arc<F>) -> Self {
        Self { frontend }
    }
}

impl<F: Frontend + Send + Sync + 'static> PageshelfWebServer for ActixPageshelfWebServer<F> {
    async fn run(&self, host: &str, port: u16) {
        use actix_web::{App, HttpServer, middleware};
        let frontend = self.frontend.clone();
        let server = HttpServer::new(move || {
            App::new()
                .wrap(middleware::Compress::default())
                .wrap(middleware::NormalizePath::trim())
                .configure(|cfg| {
                    let frontend = frontend.clone();
                    setup_service_config::<F>(cfg, frontend);
                })
        })
        .bind((host, port))
        .expect("Failed to bind server");
        server.run().await.expect("Failed to run server");
    }
}

pub fn setup_service_config<F: Frontend + 'static>(
    web_config: &mut ServiceConfig,
    frontend: Arc<F>,
) -> &mut ServiceConfig {
    web_config.app_data(web::Data::new(RoutingState { frontend }));
    //.wrap(middleware::NormalizePath::trim())
    web_config.configure(|f| {
        register_routes_to_config::<F>(f);
    });

    web_config
}
