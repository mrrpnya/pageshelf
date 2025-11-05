use std::sync::Arc;

use actix_web::web::{self, ServiceConfig};

use crate::frontend::Frontend;

//pub mod pages;
pub mod server;

/// This serves as state for the Actix server.
pub struct RoutingState<F: Frontend> {
    pub frontend: Arc<F>,
}

/* -------------------------------------------------------------------------- */
/*                                Registration                                */
/* -------------------------------------------------------------------------- */

/// Register default routes for the server to an Actix configuration.
pub fn register_routes_to_config<F: Frontend + 'static>(
    config: &mut ServiceConfig,
) -> &mut ServiceConfig {
    config.route("/{tail:.*}", web::get().to(server::request_page::<F>))
}
