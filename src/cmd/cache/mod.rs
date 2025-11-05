use clap::{Parser, Subcommand};
use color_eyre::eyre::{self, Context, ContextCompat};
use pageshelf::{conf::ServerConfig, provider::cache::cache_from_config, test_cache};
use std::fmt;
use tracing::{Level, error, info, instrument, span, warn};
pub mod check;
pub mod purge;

use crate::{
    app::PageshelfApp,
    cmd::{
        Cli,
        cache::{check::CmdCacheCheckArgs, purge::CmdCachePurgeArgs},
    },
};

#[derive(Parser, Debug)]
#[command(about = "Perform cache management operations")]
pub struct CmdCacheArgs {
    #[command(subcommand)]
    pub subcommand: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Purge(CmdCachePurgeArgs),
    Check(CmdCacheCheckArgs),
}

impl Cli {
    pub async fn cmd_cache(&self, args: &CmdCacheArgs) -> Result<(), eyre::Report> {
        match &args.subcommand {
            Commands::Purge(args) => self.cmd_cache_purge(args).await,
            Commands::Check(args) => self.cmd_cache_check(args).await,
        }
    }
}
