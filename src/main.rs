use std::sync::Arc;

use actix_web::{
    App, HttpServer, Result,
    middleware::{self, NormalizePath},
};
use chrono::{Datelike, Local};
use clap::Command;
use config::{Config, File};
use fern::colors::{Color, ColoredLevelConfig};
use log::{Level, debug, error, info, warn};
use minijinja::Environment;
use pageshelf::{
    PageSource, PageSourceFactory,
    conf::ServerConfig,
    frontend::{setup_service_config, templates::templates_from_builtin},
};

#[cfg(feature = "forgejo")]
use pageshelf::provider::ForgejoProviderFactory;

use pageshelf::conf::ServerConfigUpstreamType;

#[cfg(feature = "redis")]
use pageshelf::provider::layers::cache::CacheLayer;

use clap::{arg, crate_authors, crate_description, crate_name, crate_version};

/* -------------------------------------------------------------------------- */
/*                                    Main                                    */
/* -------------------------------------------------------------------------- */

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("{} v{}", crate_name!(), crate_version!());
    println!("Copyright {}", crate_authors!());
    println!("Licensed under the MIT License");
    println!("------------------------------\n");

    print_seasonal_message();

    let cmd = Command::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!(","))
        .about(crate_description!())
        .arg(arg!(-c --config <FILE> "Path to a config file").required(false))
        .arg(arg!(-d --debug "Enables debug information").required(false))
        //.arg(arg!(-l --log_level "Sets the logging level").required(false))
        .get_matches();

    if let Err(e) = setup_logger(cmd.get_flag("debug")) {
        eprintln!("Failed to initialize logger: {}", e);
        return Ok(()); // TODO: Use Err()
    }

    debug!("Debug logging is enabled.");

    let mut settings_builder = Config::builder();
    if let Some(v) = cmd.get_one::<String>("config") {
        settings_builder = settings_builder.add_source(File::with_name(v));
    } else {
        warn!("No configuration file was specified; Only environment variables will be used.")
    }

    settings_builder =
        settings_builder.add_source(config::Environment::with_prefix("page").separator("_"));

    let settings = match settings_builder.build() {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to build config: {}", e);
            return Ok(()); // TODO: Use Err()
        }
    };

    let config = match settings.try_deserialize::<ServerConfig>() {
        Ok(v) => v,
        Err(e) => panic!("Failed to deserialize server configuration: {}", e),
    };

    let templates = templates_from_builtin();

    match config.upstream.r#type {
        #[cfg(feature = "forgejo")]
        ServerConfigUpstreamType::Forgejo => {
            match ForgejoProviderFactory::from_config(config.clone()) {
                Some(factory) => {
                    #[cfg(feature = "redis")]
                    use pageshelf::provider::cache::RedisCache;

                    #[cfg(feature = "redis")]
                    let redis = CacheLayer::from_cache(
                        RedisCache::new(&config.cache.address, config.cache.port, config.cache.ttl)
                            .unwrap(),
                    );
                    #[cfg(feature = "redis")]
                    if config.cache.enabled {
                        use log::info;

                        info!("Redis is enabled");
                        let factory = factory.wrap(redis);
                        return run_server(factory.build(), config, templates).await;
                    }
                    run_server(factory.build(), config, templates).await
                }
                None => {
                    log::error!("The configuration failed to provide a valid Forgejo provider.");
                    return Ok(());
                }
            }
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                                Major Actions                               */
/* -------------------------------------------------------------------------- */

fn setup_logger(debug: bool) -> Result<(), fern::InitError> {
    let colors = ColoredLevelConfig::new()
        .info(Color::BrightGreen)
        .error(Color::BrightRed)
        .warn(Color::BrightYellow)
        .debug(Color::Magenta);

    let bold_code = "\x1b[1m";
    let reset_code = "\x1b[0m";

    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{}][{}{}{}]{} - {}",
                Local::now().format("%H:%M:%S"),
                if record.level() <= Level::Warn {
                    bold_code
                } else {
                    ""
                },
                colors.color(record.level()),
                if record.level() <= Level::Warn {
                    reset_code
                } else {
                    ""
                },
                if debug && let Some(file) = record.file_static() {
                    format!("[{}:{}]", file, record.line().unwrap_or(0),)
                } else {
                    "".to_string()
                },
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

async fn run_server<PS: PageSource + Sync + Send + 'static>(
    page_source: PS,
    config: ServerConfig,
    templates: Environment<'static>,
) -> std::io::Result<()> {
    let page_source = Arc::new(page_source);
    let port = config.port;
    let resolver = config.url_resolver();
    HttpServer::new(move || {
        let config = config.clone();
        let page_source = page_source.clone();
        let templates = templates.clone();
        let resolver = resolver.clone();
        App::new()
            .wrap(NormalizePath::trim())
            .wrap(middleware::Compress::default())
            .configure(move |f| {
                setup_service_config(f, &config, page_source, resolver, Some(templates));
            })
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}

// A little seasonal message, because why not
fn print_seasonal_message() {
    let now = Local::now();

    match (now.month(), now.day()) {
        (3, 17) => {
            println!("🍀 Happy Saint Patrick's Day!")
        }
        (5, _) => {
            println!("🌈 Happy Pride Month!");
            println!("❤️🧡💛💚💙💜🩷🤍🩵🖤🤎");
        }
        (10, 31) => {
            println!("🎃 Happy Halloween!")
        }
        (12, 25) => {
            println!("❄️ Merry Christmas!")
        }
        _ => {}
    }
}
