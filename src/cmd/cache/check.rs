use clap::Parser;
use color_eyre::{
    Section,
    eyre::{self, Context, ContextCompat},
};
use pageshelf::{Cache, CacheConnection, provider::cache::cache_from_config, test_cache};
use std::{
    io::{self, Write},
    time::Instant,
};
use tracing::{error, info, warn};

use crate::cmd::Cli;

#[derive(Parser, Debug)]
#[command(about = "Ensures that the cache is working correctly")]
pub struct CmdCacheCheckArgs {}

impl Cli {
    pub async fn cmd_cache_check(&self, args: &CmdCacheCheckArgs) -> Result<(), eyre::Report> {
        let cfg = self.get_config().await?;

        let cache = cache_from_config(&cfg)
            .wrap_err("Failed to set up cache from config")
            .suggestion("Check if your caching configuration is incorrect or malformed")?
            .wrap_err("No cache configured")
            .suggestion(
                "Check your configuration and ensure cache configuration is present and enabled",
            )?;

        println!("🔍 Testing cache...");
        test_cache(cache)
            .await
            .wrap_err("Cache check failed")
            .suggestion("Check that the cache is online and responding correctly")?;
        println!("✅ Cache tested succesfully");

        Ok(())
    }
}
