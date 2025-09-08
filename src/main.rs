use actix_web::{App, HttpServer, Result, middleware::NormalizePath};
use clap::Command;
use config::{Config, File};
use fern::colors::{Color, ColoredLevelConfig};
use log::debug;
use minijinja::Environment;
use pageshelf::{
    conf::ServerConfig, frontend::setup_service_config,
    frontend::templates::templates_from_builtin, page::PageSourceFactory,
};

#[cfg(feature = "forgejo")]
use pageshelf::backend::ForgejoProviderFactory;

use pageshelf::conf::ServerConfigUpstreamType;

#[cfg(feature = "redis")]
use pageshelf::backend::layers::redis::RedisLayer;

use clap::{arg, crate_authors, crate_description, crate_name, crate_version};

/* -------------------------------------------------------------------------- */
/*                                    Main                                    */
/* -------------------------------------------------------------------------- */

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let cmd = Command::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!(","))
        .about(crate_description!())
        .arg(arg!(-c --config <FILE> "Path to a config file").required(false))
        .arg(arg!(-d --debug "Enables debug information").required(false))
        //.arg(arg!(-l --log_level "Sets the logging level").required(false))
        .get_matches();

    let _ = setup_logger(cmd.get_flag("debug"));
    debug!("If you're seeing this, debug logging is enabled.");

    let mut settings_builder = Config::builder();
    match cmd.get_one::<String>("config") {
        Some(v) => {
            settings_builder = settings_builder.add_source(File::with_name(v));
        }
        None => {}
    }
    // Add in settings from the environment (with a prefix of APP)
    // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
    settings_builder =
        settings_builder.add_source(config::Environment::with_prefix("page").separator("_"));

    let settings = settings_builder.build().unwrap();

    let config = match settings.try_deserialize::<ServerConfig>() {
        Ok(v) => v,
        Err(e) => panic!("Failed to deserialize server configuration: {}", e),
    };

    let templates = templates_from_builtin();

    match config.upstream.r#type {
        #[cfg(feature = "forgejo")]
        ServerConfigUpstreamType::Forgejo => {
            match ForgejoProviderFactory::from_config(config.clone()) {
                Ok(factory) => {
                    #[cfg(feature = "redis")]
                    let redis = RedisLayer::from_config(&config).unwrap();
                    #[cfg(feature = "redis")]
                    match config.redis.enabled {
                        true => {
                            use log::info;

                            info!("Redis is enabled");
                            let factory = factory.wrap(redis);
                            return run_server(factory, config, templates).await;
                        }
                        false => {}
                    }
                    run_server(factory, config, templates).await
                }
                Err(_) => {
                    log::error!("Failed to generate Forgejo provider via configs");
                    return Ok(());
                }
            }
        }
        // This will be used as a fallback, should no feature be available.
        _ => {
            log::error!("Failed to determine provider to use");
            return Ok(());
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                                Major Actions                               */
/* -------------------------------------------------------------------------- */

fn setup_logger(debug: bool) -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            let colors = ColoredLevelConfig::new()
                .info(Color::BrightGreen)
                .error(Color::BrightRed)
                .warn(Color::BrightYellow)
                .debug(Color::Magenta);
            out.finish(format_args!(
                "[{}] {}",
                colors.color(record.level()),
                message
            ))
        })
        .level(match debug {
            true => log::LevelFilter::Debug,
            false => log::LevelFilter::Info,
        })
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}

async fn run_server<'a, PS: PageSourceFactory + Sync + Send + 'static>(
    page_factory: PS,
    config: ServerConfig,
    templates: Environment<'static>,
) -> std::io::Result<()>
where
    <PS as PageSourceFactory>::Source: 'static,
{
    let port = config.port;
    HttpServer::new(move || {
        let config = config.clone();
        let page_factory = page_factory.clone();
        let templates = templates.clone();
        App::new().wrap(NormalizePath::trim()).configure(move |f| {
            setup_service_config(f, &config, page_factory, Some(templates));
        })
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
