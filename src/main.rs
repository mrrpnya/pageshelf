pub mod app;
pub mod cmd;

use clap::Parser;
use color_eyre::{
    config::{HookBuilder, Theme},
    eyre::Report,
};
use time::Month;
use tracing::{debug, info};

use clap::{crate_authors, crate_name, crate_version};
use tracing::{Level, instrument};

use crate::cmd::Cli;

fn print_banner(lines: &[String]) {
    // Compute the maximum content width (not including padding)
    let max_content_width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let padding = 2; // spaces between border and text
    let total_width = max_content_width + padding * 2;

    // Top border
    println!("╔{}╗", "═".repeat(total_width));

    // Centered text lines
    for line in lines.iter() {
        let line_len = line.chars().count();
        let total_space = total_width - line_len;

        let left_pad = total_space / 2;
        let right_pad = total_space - left_pad;

        println!(
            "║{}{}{}║",
            " ".repeat(left_pad),
            line,
            " ".repeat(right_pad)
        );
    }

    // Bottom border
    println!("╚{}╝\n", "═".repeat(total_width));
}

/* -------------------------------------------------------------------------- */
/*                                    Main                                    */
/* -------------------------------------------------------------------------- */

#[actix_web::main]
#[instrument]
async fn main() -> Result<(), Report> {
    HookBuilder::default().theme(Theme::dark()).install()?;
    let cli = Cli::parse();

    if !cli.quiet {
        let banner = vec![
            format!("{} v{}", crate_name!(), crate_version!()),
            format!("Copyright (c) {}", crate_authors!()),
            "Licensed under the terms of the MIT License.".to_string(),
            "See the LICENSE file in the project root for full details.".to_string(),
        ];

        print_banner(&banner);

        if let Some(msg) = seasonal_message() {
            msg.iter().for_each(|f| println!("{f}"));
        }

        println!();
    }

    if cli.debug {
        println!("Preparing to set up environment:");
    }

    let level = match cli.quiet {
        false => match cli.debug {
            true => Level::DEBUG,
            false => Level::INFO,
        },
        true => Level::WARN,
    };

    if cli.debug {
        println!("- Determined log level to be {level}.");
        match cli.pretty {
            true => println!("- Pretty printing enabled"),
            false => println!("- Pretty printing disabled"),
        }
    }

    if !cli.quiet {
        println!();
    }

    pageshelf::log::setup_logger(level, cli.pretty);

    if cli.debug {
        info!("Logger initialized.");
    }

    debug!("Debug logging is enabled.");

    cli.cmd_auto().await?;

    Ok(())
}

// because why not
fn seasonal_message() -> Option<Vec<String>> {
    let now = time::OffsetDateTime::now_local().unwrap();

    match (now.month(), now.day()) {
        (Month::March, 17) => Some(vec!["Happy Saint Patrick's Day!".to_string()]),
        (Month::June, _) => Some(vec![
            "Happy Pride Month!".to_string(),
            "❤️🧡💛💚💙💜🩷🤍🩵🖤🤎".to_string(),
        ]),
        (Month::October, 31) => Some(vec!["Happy Halloween!".to_string()]),
        (Month::December, 25) => Some(vec!["Merry Christmas!".to_string()]),
        _ => None,
    }
}
