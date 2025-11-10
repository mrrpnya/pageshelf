//! Route registration and server state used by the Actix integration.

use std::sync::Arc;

use actix_web::web::{self, ServiceConfig};
use pageshelf_frontend::Frontend;

//pub mod pages;
pub mod server;

/// Shared state attached to Actix handlers.
///
/// The state currently only contains the `Frontend` instance which is
/// used to fulfil incoming requests.
pub struct RoutingState<F: Frontend> {
    /// Shared frontend used to build responses.
    pub frontend: Arc<F>,
}

/// Register the default routes with the provided Actix `ServiceConfig`.
///
/// This will register a catch-all GET route which delegates handling to
/// `server::main_route` so the frontend can resolve pages and assets.
pub fn register_routes_to_config<F: Frontend + 'static>(
    config: &mut ServiceConfig,
) -> &mut ServiceConfig {
    config.route("/{tail:.*}", web::get().to(server::main_route::<F>))
}
