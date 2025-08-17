use std::str::FromStr;

use actix_web::{middleware::NormalizePath, web, App, HttpServer, Result};
use clap::Command;
use config::{Config, File};
use fern::colors::{Color, ColoredLevelConfig};
use forgejo_api::{Auth, Forgejo};
use minijinja::Environment;
use pageshelf::{
    conf::ServerConfig,
    page::{PageSource, PageSourceConfigurator},
    providers::forgejo::ForgejoProvider,
    routes::{self, RouteSharedData},
    templates::templates_from_builtin,
};

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
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log")?)
        .apply()?;
    Ok(())
}

use clap::{arg, crate_authors, crate_description, crate_name, crate_version};

async fn run_server<'a, PS: PageSourceConfigurator + Sync + Send>(
    port: u16,
    config: ServerConfig,
    templates: Environment<'static>,
) -> std::io::Result<()>
where
    <PS as PageSourceConfigurator>::Source: 'static,
{
    HttpServer::new(move || {
        let pages = config.upstream.branches.clone();
        let config = config.clone();
        App::new()
            .wrap(NormalizePath::trim())
            .app_data(web::Data::new(RouteSharedData {
                provider: PS::configure(&config),
                jinja: templates.clone(),
                config: config,
            }))
            //.wrap(middleware::NormalizePath::trim())
            .configure(|f| {
                routes::register_to_service_config::<ForgejoProvider>(f);
            })
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await
}

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

    let port = config.general.port.clone();

    run_server::<ForgejoProvider>(port, config, templates).await
}
