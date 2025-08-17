use std::str::FromStr;

use actix_web::{App, HttpServer, Result, web};
use clap::Command;
use config::{Config, File};
use fern::colors::{Color, ColoredLevelConfig};
use forgejo_api::{Auth, Forgejo};
use log::info;
use pageshelf::{
    conf::ServerConfig, providers::forgejo::ForgejoProvider, routes::{self, RouteSharedData}, templates::templates_from_builtin
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
use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ServerJinjaContext {
    name: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let cmd = Command::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!(","))
        .about(crate_description!())
        .arg(arg!(-c --config <FILE> "Path to a config file").required(false))
        .arg(arg!(-l --log_level "Sets the logging level").required(false))
        .get_matches();

    let _ = setup_logger();

    let mut settings_builder = Config::builder()
        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        .add_source(config::Environment::with_prefix("PAGE"));

    match cmd.get_one::<String>("config") {
        Some(v) => {settings_builder = settings_builder.add_source(File::with_name(v));},
        None => {}
    }
    
    let settings = settings_builder.build()
        .unwrap();

    let config = match settings.try_deserialize::<ServerConfig>() {
        Ok(v) => v,
        Err(e) => panic!("Failed to deserialize server configuration: {}", e),
    };

    HttpServer::new(move || {
        let pages = vec!["pages".to_string()];

        let forgejo = Forgejo::new(
            Auth::None,
            url::Url::from_str(&config.upstream.url).unwrap(),
        )
        .unwrap();

        App::new()
            .app_data(web::Data::new(RouteSharedData {
                provider: routes::UpstreamProviderType::Forgejo(ForgejoProvider::direct(
                    forgejo,
                    Some(pages),
                )),
                jinja: templates_from_builtin(),
                config: config.clone()
            }))
            .service(routes::pages::get_page_or)
            .service(routes::pages::get_page_orf)
            .service(routes::server::get_index)
            .service(routes::server::get_favicon_svg)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
