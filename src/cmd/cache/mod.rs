use clap::{Parser, Subcommand};
use color_eyre::eyre::{self};
pub mod check;
pub mod purge;

use crate::cmd::{
    Cli,
    cache::{check::CmdCacheCheckArgs, purge::CmdCachePurgeArgs},
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
