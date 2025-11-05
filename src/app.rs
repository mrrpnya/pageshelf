use std::sync::Arc;

#[cfg(feature = "forgejo")]
use pageshelf::conf::ServerConfigUpstreamType;
use pageshelf::{
    conf::ServerConfig,
    frontend::{
        DefaultFrontend,
        renderer::{FrontendRenderer, jinja::JinjaFrontendRenderer},
    },
    project::source::ProjectSource,
    server::{PageshelfWebServer, actix::ActixPageshelfWebServer},
};

use tracing::error;

pub struct PageshelfApp {
    config: ServerConfig,
}

impl PageshelfApp {
    pub fn from_server_config(config: ServerConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    pub async fn run_checks(&self) {
        self.run(true).await;
    }

    pub async fn run(&self, dry: bool) {
        let renderer = JinjaFrontendRenderer::from_server_config(&self.config);
        match self.config.upstream.r#type {
            #[cfg(feature = "forgejo")]
            ServerConfigUpstreamType::Forgejo => {
                use pageshelf::provider::ForgejoProviderFactory;

                match ForgejoProviderFactory::from_config(&self.config) {
                    Some(factory) => {
                        #[cfg(feature = "redis")]
                        use pageshelf::provider::cache::RedisCache;

                        if self.config.cache.enabled {
                            use tracing::info;
                            #[cfg(not(feature = "redis"))]
                            {
                                tracing::warn!(
                                    "Caching was enabled, but no cache providers are available!"
                                );
                            }
                            #[cfg(feature = "redis")]
                            {
                                use pageshelf::{
                                    project::layer::ProjectSourceBuilder,
                                    provider::layers::cache::CacheLayer,
                                };

                                let redis = CacheLayer::from_cache(
                                    RedisCache::new(
                                        &self.config.cache.address,
                                        self.config.cache.port,
                                        self.config.cache.ttl,
                                    )
                                    .unwrap(),
                                );
                                info!("Redis is enabled");
                                let factory = factory.wrap(redis);
                                if !dry {
                                    use pageshelf::project::layer::ProjectSourceBuilder;

                                    return Self::_run_server(
                                        factory.build(),
                                        self.config.clone(),
                                        renderer,
                                    )
                                    .await;
                                }
                                return;
                            }
                        }
                        if !dry {
                            use pageshelf::project::layer::ProjectSourceBuilder;

                            Self::_run_server(factory.build(), self.config.clone(), renderer).await
                        }
                    }
                    None => {
                        error!("The configuration failed to provide a valid Forgejo provider.");
                    }
                }
            }
        }
    }

    async fn _run_server<
        PS: ProjectSource + Sync + Send + 'static,
        RD: FrontendRenderer + 'static,
    >(
        source: PS,
        config: ServerConfig,
        renderer: RD,
    ) {
        let source = source;
        let port = config.port;
        let resolver = config.url_resolver();
        let frontend = Arc::new(DefaultFrontend::new(renderer, resolver, source));
        let server = ActixPageshelfWebServer::new(frontend);

        server.run("0.0.0.0", port).await;
    }
}
