use actix_web::{App, HttpServer, Result, middleware::NormalizePath};
use clap::Command;
use config::{Config, File};
use fern::colors::{Color, ColoredLevelConfig};
use minijinja::Environment;
use pageshelf::{
    conf::ServerConfig, page::PageSourceFactory, providers::ForgejoProviderFactory,
    routes::setup_service_config, templates::templates_from_builtin,
};

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
        //.arg(arg!(-l --log_level "Sets the logging level").required(false))
        .get_matches();

    let _ = setup_logger();

    let mut settings_builder = Config::builder()
        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        .add_source(config::Environment::with_prefix("page").separator("_"));

    match cmd.get_one::<String>("config") {
        Some(v) => {
            settings_builder = settings_builder.add_source(File::with_name(v));
        }
        None => {}
    }

    let settings = settings_builder.build().unwrap();

    let config = match settings.try_deserialize::<ServerConfig>() {
        Ok(v) => v,
        Err(e) => panic!("Failed to deserialize server configuration: {}", e),
    };

    let templates = templates_from_builtin();

    let factory = match ForgejoProviderFactory::from_config(config.clone()) {
        Ok(v) => v,
        Err(_) => {
            return Ok(());
        }
    };

    run_server(factory, config, templates).await
}

/* -------------------------------------------------------------------------- */
/*                                Major Actions                               */
/* -------------------------------------------------------------------------- */

fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            let colors = ColoredLevelConfig::new()
                .info(Color::BrightGreen)
                .error(Color::BrightRed)
                .warn(Color::BrightYellow);
            out.finish(format_args!(
                "[{}] {}",
                colors.color(record.level()),
                message
            ))
        })
        .level(log::LevelFilter::Info)
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
        // TODO: See if cloning can be reduced here? It's done every worker.
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
