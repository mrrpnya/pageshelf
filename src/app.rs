use std::sync::Arc;

use color_eyre::eyre;
#[cfg(feature = "forgejo")]
use pageshelf::conf::ServerConfigUpstreamType;
use pageshelf::{
    DefaultFrontend, WebServer,
    actix::ActixWebServer,
    conf::ServerConfig,
    domain::UpstreamPageDomainResolver,
    event::EventBus,
    renderer::{Renderer, jinja::JinjaRenderer},
    resolution::{DirectoryPageResolver, PageResolver, SubdomainPageResolver},
    upstream::Upstream,
};

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
        let _ = self.run(true, None, None).await;
    }

    pub async fn run(
        &self,
        dry: bool,
        host_override: Option<&str>,
        port_override: Option<u16>,
    ) -> Result<(), eyre::Report> {
        let subdomain_domains = self
            .config
            .pages_domains
            .clone()
            .unwrap_or_default()
            .iter()
            .map(|u| u.domain().unwrap().to_string())
            .collect::<Vec<_>>();

        let renderer = JinjaRenderer::default();

        let resolver =
            DirectoryPageResolver::new(Some("pages".to_string()), Some("pages".to_string()))
                .with_layer(SubdomainPageResolver::new(
                    Some(subdomain_domains),
                    Some("pages".to_string()),
                    Some("pages".to_string()),
                ));

        let event_bus = EventBus::default();
        match self.config.upstream.r#type {
            #[cfg(feature = "forgejo")]
            ServerConfigUpstreamType::Forgejo => {
                use pageshelf_provider_upstream_forgejo::{ForgejoUpstream, ForgejoUpstreamConfig};

                let cfg = ForgejoUpstreamConfig {
                    url: self.config.upstream.url.clone(),
                    branches: self.config.upstream.branches.clone(),
                    scan_interval: None,
                };

                match ForgejoUpstream::create(&cfg, event_bus.clone()) {
                    Ok(upstream) => {
                        #[cfg(feature = "redis")]
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
                                use pageshelf::upstream::CachedUpstream;
                                use pageshelf_provider_cache_redis::RedisCache;

                                let redis = RedisCache::new(
                                    &self.config.cache.address,
                                    self.config.cache.port,
                                    self.config.cache.ttl,
                                )
                                .unwrap();

                                info!("Redis is enabled");
                                let cached_upstream =
                                    CachedUpstream::wrap(redis, upstream, event_bus.clone());
                                let resolver = resolver.with_layer(
                                    UpstreamPageDomainResolver::new(
                                        cached_upstream.clone(),
                                        event_bus.clone(),
                                    )
                                    .await,
                                );
                                if !dry {
                                    // bind host/port overrides are applied in _run_server below
                                    let host = host_override.unwrap_or("0.0.0.0");
                                    let port = port_override.unwrap_or(self.config.port);

                                    return Self::_run_server(
                                        cached_upstream,
                                        self.config.clone(),
                                        resolver,
                                        renderer,
                                        host,
                                        port,
                                    )
                                    .await;
                                }
                                return Ok(());
                            }
                        }
                        if !dry {
                            let host = host_override.unwrap_or("0.0.0.0");
                            let port = port_override.unwrap_or(self.config.port);

                            return Self::_run_server(
                                Arc::from(upstream),
                                self.config.clone(),
                                resolver,
                                renderer,
                                host,
                                port,
                            )
                            .await;
                        }

                        Ok(())
                    }
                    Err(_) => Err(eyre::Report::msg(
                        "The configuration failed to provide a valid Forgejo provider",
                    )),
                }
            }
        }
    }

    async fn _run_server<
        US: Upstream + Sync + Send + 'static,
        RD: Renderer + 'static,
        PR: PageResolver + Sync + Send + 'static,
    >(
        upstream: Arc<US>,
        config: ServerConfig,
        resolver: PR,
        renderer: RD,
        host: &str,
        port: u16,
    ) -> Result<(), eyre::Report> {
        let frontend = Arc::new(DefaultFrontend::new(renderer, resolver, upstream));
        let server = ActixWebServer::new(frontend)?;

        server.run(host, port).await
    }
}
