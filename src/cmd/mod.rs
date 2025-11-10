pub mod cache;
pub mod check;
pub mod serve;
pub mod styles;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use color_eyre::{
    Section,
    eyre::{self, Context, ContextCompat},
};
use config::{Config, File};
use tracing::info;

use crate::cmd::styles::get_styles;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(styles=get_styles())]
#[command(next_line_help = true)]
pub struct Cli {
    #[arg(short, long, help = "", global = true)]
    pub config: Option<PathBuf>,
    /// Suppress the startup banner (ignores quiet)
    #[arg(long, global = true, help = "Suppress the startup banner output")]
    pub no_banner: bool,
    /// Explicit log level (trace|debug|info|warn|error). Overrides --debug/--quiet when present.
    #[arg(
        long,
        global = true,
        help = "Set explicit logging level (trace|debug|info|warn|error)"
    )]
    pub log_level: Option<String>,
    #[arg(
        short,
        long,
        global = true,
        default_value = "false",
        default_missing_value = "true",
        help = "Enable debug logging?"
    )]
    pub debug: bool,
    #[arg(
        short,
        long,
        global = true,
        default_value = "false",
        default_missing_value = "true",
        help = "Reduce unimportant (non-debug) logging?"
    )]
    pub quiet: bool,
    #[arg(
        short,
        long,
        global = true,
        default_value = "false",
        default_missing_value = "true",
        help = "Expand log messages to be more readable at the cost of compactness?"
    )]
    pub pretty: bool,
    #[command(subcommand)]
    pub subcommand: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Check(check::CmdCheckArgs),
    Serve(serve::CmdServeArgs),
    Cache(cache::CmdCacheArgs),
}

impl Cli {
    pub async fn get_config(&self) -> Result<Config, eyre::Report> {
        let mut settings_builder = Config::builder();
        if let Some(v) = &self.config {
            let path = v
                .to_str()
                .wrap_err("Failed to interpret the path of the config file")
                .suggestion("check your --config argument")?;
            settings_builder = settings_builder.add_source(File::with_name(path));
        } else {
            info!("No configuration file was specified; Only environment variables will be used.")
        }

        settings_builder =
            settings_builder.add_source(config::Environment::with_prefix("page").separator("_"));

        settings_builder
            .build()
            .wrap_err("Failed to build config")
            .suggestion("Check the formatting of your configuration")
    }

    pub async fn cmd_auto(&self) -> Result<(), eyre::Report> {
        match &self.subcommand {
            Commands::Check(args) => self.cmd_check(args).await,
            Commands::Serve(args) => self.cmd_serve(args).await,
            Commands::Cache(args) => self.cmd_cache(args).await,
        }
    }
}
