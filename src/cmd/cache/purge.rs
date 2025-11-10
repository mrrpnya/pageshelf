use clap::Parser;
use color_eyre::eyre::{self, Context};
use pageshelf::{
    cache::{Cache, CacheConnection},
    cache_combiner::cache_from_config,
};
use std::{
    io::{self, Write},
    time::Instant,
};

use crate::cmd::Cli;

#[derive(Parser, Debug)]
#[command(about = "Purges all data from the cache")]
pub struct CmdCachePurgeArgs {
    /// Skip confirmation prompt
    #[arg(short = 'y', long = "yes")]
    pub yes: bool,
}

impl Cli {
    pub async fn cmd_cache_purge(&self, args: &CmdCachePurgeArgs) -> Result<(), eyre::Report> {
        let cfg = self.get_config().await?;

        // Proceed with purge
        match cache_from_config(&cfg).wrap_err("Could not get cache")? {
            Some(cache) => {
                // Ask for confirmation unless --yes is provided
                if !args.yes {
                    print!("⚠️  This will permanently delete all cached data. Continue? [y/N]: ");
                    io::stdout().flush().ok();

                    let mut input = String::new();
                    io::stdin()
                        .read_line(&mut input)
                        .wrap_err("Failed to read user input")?;

                    let input = input.trim().to_lowercase();
                    if input != "y" && input != "yes" {
                        println!("Aborted by user.");
                        return Ok(());
                    }
                }

                let begin = Instant::now();

                let mut conn = cache
                    .connect()
                    .await
                    .wrap_err("Failed to connect to cache")?;

                let count = conn.purge().await.wrap_err("Failed to purge cache")?;

                match count {
                    Some(count) => println!(
                        "Purged cache in {}ms ({count} keys deleted)",
                        begin.elapsed().as_millis()
                    ),
                    None => println!("Purged cache in {}ms", begin.elapsed().as_millis()),
                }
            }
            None => {
                println!("⛔ No cache available!");
            }
        }

        Ok(())
    }
}
