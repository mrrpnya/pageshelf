use std::sync::Arc;

use actix_web::web::{self, ServiceConfig};
use minijinja::Environment;

use crate::{PageSource, conf::ServerConfig, resolver::UrlResolver};

pub mod pages;
pub mod server;

/// This serves as state for the Actix server.
pub struct RoutingState<'a, PS: PageSource, UR: UrlResolver> {
    pub provider: Arc<PS>,
    pub config: ServerConfig,
    pub jinja: Environment<'a>,
    pub resolver: UR,
}

/* -------------------------------------------------------------------------- */
/*                                Registration                                */
/* -------------------------------------------------------------------------- */

/// Register default routes for the server to an Actix configuration.
pub fn register_routes_to_config<PS: PageSource + 'static, UR: UrlResolver + 'static>(
    config: &mut ServiceConfig,
) -> &mut ServiceConfig {
    config
        .service(server::get_favicon_webp)
        .route("/{tail:.*}", web::get().to(server::get_index::<PS, UR>))
}
