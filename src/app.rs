use std::{net::SocketAddr, sync::Arc, time::Duration};

use color_eyre::{
    Section,
    eyre::{self, Context},
};
use config::Config;
use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_process::Collector;
use pageshelf::conf::ServerCacheBackend;
#[cfg(feature = "forgejo")]
use pageshelf::conf::ServerConfigUpstreamType;
use pageshelf::{
    DefaultFrontend, WebServer,
    actix::ActixWebServer,
    conf::ServerConfig,
    event::EventBus,
    renderer::{Renderer, jinja::JinjaRenderer},
    resolution::{DirectoryPageResolver, PageResolver, SubdomainPageResolver},
    upstream::Upstream,
};
use tracing::info;

pub struct PageshelfApp {
    config: ServerConfig,
    raw_config: Config,
}

impl PageshelfApp {
    pub fn from_server_config(config: ServerConfig, raw_config: Config) -> Self {
        Self { config, raw_config }
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

        let renderer = JinjaRenderer::new(
            None,
            self.config.name.clone(),
            self.config.description.clone(),
            self.config
                .domain
                .clone()
                .map(|f| f.domain().unwrap().to_string()),
            None,
            self.config.upstream.default_branch.clone(),
        );

        let resolver =
            DirectoryPageResolver::new(Some("pages".to_string()), Some("pages".to_string()))
                .with_layer(SubdomainPageResolver::new(
                    Some(subdomain_domains),
                    Some("pages".to_string()),
                    Some("pages".to_string()),
                ));

        // === Metrics

        if self.config.metrics.enabled {
            info!("Metrics is enabled on port {}", self.config.metrics.port);
            let addr: SocketAddr = format!("0.0.0.0:{}", self.config.metrics.port).parse()?;
            let prometheus = PrometheusBuilder::new().with_http_listener(addr);
            prometheus
                .install()
                .wrap_err("Failed to build Prometheus metrics exporter")
                .suggestion("Check your metrics configuration")
                .suggestion("Check if the port is in use")?;
            info!("Prometheus set up");

            // Process metrics (cpu/mem/etc)
            tokio::spawn(async move {
                let collector = Collector::default();
                collector.describe();
                loop {
                    collector.collect();
                    tokio::time::sleep(Duration::from_millis(750)).await;
                }
            });
        }

        // === Provision
        let event_bus = EventBus::default();
        match self.config.upstream.r#type {
            #[cfg(feature = "forgejo")]
            ServerConfigUpstreamType::Forgejo => {
                use pageshelf_provider_upstream_forgejo::{
                    ForgejoRawSource, ForgejoUpstreamConfig,
                };

                let cfg = ForgejoUpstreamConfig {
                    url: self.config.upstream.url.clone(),
                    branches: self.config.upstream.branches.clone(),
                    scan_interval: None,
                };

                match ForgejoRawSource::create(&cfg, cfg.branches.clone()) {
                    Ok(source) => {
                        use pageshelf::upstream::managers::PolledSource;

                        let upstream = PolledSource::start(
                            Arc::from(source),
                            self.config.upstream.poll_interval.unwrap_or(240),
                            event_bus.clone(),
                        );
                        #[cfg(feature = "redis")]
                        if self.config.cache.enabled {
                            #[cfg(feature = "redis")]
                            use tracing::info;
                            match self.config.cache.backend {
                                #[cfg(feature = "redis")]
                                ServerCacheBackend::Redis => {
                                    use pageshelf::{
                                        domain::UpstreamPageDomainResolver,
                                        resolution::{RegexPageFilter, RegexPageFilterRules},
                                        upstream::managers::CachedSource,
                                    };
                                    use pageshelf_provider_cache_redis::RedisCache;

                                    let redis = RedisCache::new(
                                        &self.config.cache.address,
                                        self.config.cache.port,
                                        self.config.cache.ttl,
                                    )
                                    .unwrap();

                                    info!("Redis is enabled");
                                    let cached_upstream =
                                        CachedSource::wrap(redis, upstream, event_bus.clone());
                                    let resolver = resolver.with_layer(
                                        UpstreamPageDomainResolver::new(
                                            cached_upstream.clone(),
                                            event_bus.clone(),
                                        )
                                        .await,
                                    );
                                    if let Some(rules) =
                                        RegexPageFilterRules::from_config(&self.raw_config)
                                    {
                                        if !dry {
                                            let filter = RegexPageFilter::new(rules);
                                            let resolver = resolver.with_filter(filter);
                                            if !dry {
                                                // bind host/port overrides are applied in _run_server below
                                                let host = host_override.unwrap_or("0.0.0.0");
                                                let port =
                                                    port_override.unwrap_or(self.config.port);

                                                return Self::_run_server(
                                                    cached_upstream,
                                                    resolver,
                                                    renderer,
                                                    host,
                                                    port,
                                                )
                                                .await;
                                            }
                                        }
                                    } else if !dry {
                                        // bind host/port overrides are applied in _run_server below
                                        let host = host_override.unwrap_or("0.0.0.0");
                                        let port = port_override.unwrap_or(self.config.port);

                                        return Self::_run_server(
                                            cached_upstream,
                                            resolver,
                                            renderer,
                                            host,
                                            port,
                                        )
                                        .await;
                                    }
                                    return Ok(());
                                }
                                #[allow(unused)] // Comes into play if no features are enabled
                                _ => {
                                    return Err(eyre::Report::msg("No cache backend")
                                        .suggestion("Check your cache configuration")
                                        .suggestion("Check feature flags for cache support"));
                                }
                            }
                        }
                        if !dry {
                            let host = host_override.unwrap_or("0.0.0.0");
                            let port = port_override.unwrap_or(self.config.port);

                            return Self::_run_server(
                                Arc::from(upstream),
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
