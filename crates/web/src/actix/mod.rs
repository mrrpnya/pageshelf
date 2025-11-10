//! Actix integration used to serve Frontend content.
//!
//! Priorities:
//! - Provide a lightweight Actix-backed server implementation
//!   that the application can start or embed.
//! - Centralise route registration and wiring of the shared `Frontend`
//!   instance so handlers remain thin and focused on request handling.

mod response;
mod routes;
use crate::WebServer;
use actix_web::web::{self, ServiceConfig};
use actix_web::{App, HttpServer, middleware};
use color_eyre::{
    Section,
    eyre::{self, Context},
};
use pageshelf_frontend::Frontend;
use routes::{RoutingState, register_routes_to_config};
use std::net::TcpListener;
use std::sync::Arc;

// TODO: Rate, etc limiting?
// TODO: Allow disabling JS execution on pages globally?

/// An Actix-web backed server implementation.
///
/// Holds a shared `Frontend` implementation and exposes methods to run
/// the server either by binding to a host/port or using an existing
/// `TcpListener`.
#[derive(Default)]
pub struct ActixWebServer<F: Frontend + Send + Sync + 'static> {
    /// Shared frontend instance used to respond to incoming requests.
    frontend: Arc<F>,
}

impl<F: Frontend + Send + Sync + 'static> ActixWebServer<F> {
    /// Create a new `ActixWebServer` around the given frontend.
    pub fn new(frontend: Arc<F>) -> Result<Self, eyre::Report> {
        Ok(Self { frontend })
    }

    /// Run the Actix server using an existing `TcpListener`.
    ///
    /// This is useful when the caller wants to bind the socket themselves
    /// (for example to reuse an existing socket or integrate with a
    /// super-server). Errors are returned as `eyre::Report`.
    pub async fn run_with_listener(&self, listener: TcpListener) -> Result<(), eyre::Report> {
        let frontend = self.frontend.clone();
        let server = HttpServer::new(move || {
            App::new()
                .wrap(middleware::Compress::default())
                .wrap(middleware::NormalizePath::trim())
                .wrap(
                    middleware::DefaultHeaders::new()
                        .add(("X-Content-Type-Options", "nosniff"))
                        .add(("X-Frame-Options", "DENY"))
                        .add(("X-XSS-Protection", "1; mode=block")),
                )
                .wrap(
                    middleware::DefaultHeaders::new()
                        .add((
                            "Content-Security-Policy",
                            "default-src 'self'; script-src 'self' style-src 'self' 'unsafe-inline'",
                        ))
                        .add(("Referrer-Policy", "same-origin")),
                ) // TODO: Error page
                .configure(|cfg| {
                    let frontend = frontend.clone();
                    Self::setup_service_routing(cfg, frontend);
                })
        })
        .listen(listener)
        .wrap_err("Failed to bind Actix web server with listener")?;

        server
            .run()
            .await
            .wrap_err("Failed to start Actix web server with listener")?;

        Ok(())
    }
}

impl<F: Frontend + Send + Sync + 'static> WebServer for ActixWebServer<F> {
    type Frontend = F;

    /// Start the server by binding to the provided host and port.
    async fn run(&self, host: &str, port: u16) -> Result<(), eyre::Report> {
        let listener = TcpListener::bind(format!("{host}:{port}"))
            .wrap_err("Failed to bind TCP listener")
            .suggestion("Check your host and port?")?;

        self.run_with_listener(listener).await
    }
}

impl<F: Frontend + Send + Sync + 'static> ActixWebServer<F> {
    /// Configure the given Actix `ServiceConfig` with all needed routing information here.
    pub fn setup_service_routing(
        service_config: &mut ServiceConfig,
        frontend: Arc<F>,
    ) -> &mut ServiceConfig {
        service_config
            .app_data(web::Data::new(RoutingState { frontend }))
            .configure(|f| {
                register_routes_to_config::<F>(f);
            })
    }
}
