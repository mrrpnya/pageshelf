use clap::Parser;
use color_eyre::eyre::{self, Context};
use pageshelf::conf::ServerConfig;
use std::fmt;
use tracing::{Level, info, instrument, span, warn};

use crate::{app::PageshelfApp, cmd::Cli};

#[derive(Parser, Debug)]
#[command(
    about = "Runs system and integration checks to find problems and help ensure things work"
)]
pub struct CmdCheckArgs {
    /// Skip network checks (DNS/connectivity)
    #[arg(long)]
    pub skip_network: bool,
    /// Skip entropy/cryptography checks
    #[arg(long)]
    pub skip_entropy: bool,
    /// Verbose output
    #[arg(long, short, default_value_t = false)]
    pub verbose: bool,
}

#[derive(serde::Serialize)]
struct CheckSummary {
    ok: usize,
    warn: usize,
    fail: usize,
    details: Vec<String>,
}

/// Represents a single check result.
#[derive(Debug)]
enum CheckStatus {
    Ok(String),
    Warn(String),
    Fail(String),
}

impl fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckStatus::Ok(msg) => write!(f, "✅ OK: {msg}"),
            CheckStatus::Warn(msg) => write!(f, "⚠️ WARN: {msg}"),
            CheckStatus::Fail(msg) => write!(f, "❌ FAIL: {msg}"),
        }
    }
}

impl Cli {
    /// Ensures that the provided setup is valid and is able to function.
    #[instrument(level = "debug")]
    pub async fn cmd_check(&self, args: &CmdCheckArgs) -> Result<(), eyre::Report> {
        info!("Performing system and configuration checks...");

        let mut results = Vec::<CheckStatus>::new();

        let cfg = self.get_config().await?;
        let sv_cfg = ServerConfig::from_config(&cfg)
            .wrap_err("No server configuration available; This is required.")?;

        tracing::info!(
            "Running on: {} ({})",
            std::env::consts::OS,
            std::env::consts::ARCH
        );

        let now = time::UtcDateTime::now();
        tracing::debug!("System time (UTC): {}", now);

        // === Sanity Checks ===
        {
            let span = span!(Level::INFO, "sanity");
            let _span_guard = span.enter();

            // Cryptography entropy
            {
                let span = span!(Level::INFO, "cryptography");
                let _span_guard = span.enter();
                if !args.skip_entropy {
                    #[cfg(unix)]
                    {
                        if let Ok(mut f) =
                            std::fs::File::open("/proc/sys/kernel/random/entropy_avail")
                        {
                            use std::io::Read;
                            let mut buf = String::new();
                            let _ = f.read_to_string(&mut buf);
                            if let Ok(entropy) = buf.trim().parse::<u32>() {
                                if entropy < 100 {
                                    warn!(
                                        "Low system entropy ({entropy}) - crypto operations may block."
                                    );
                                    results.push(CheckStatus::Warn(format!(
                                        "Low system entropy ({entropy})"
                                    )));
                                } else {
                                    info!("Sufficient system entropy ({entropy})");
                                    results.push(CheckStatus::Ok(format!(
                                        "Sufficient system entropy ({entropy})"
                                    )));
                                }
                            }
                        }
                    }
                } else {
                    info!("Skipping entropy checks as requested");
                }
            }

            // Network connectivity
            {
                let span = span!(Level::INFO, "network");
                let _span_guard = span.enter();

                let dns_server = "1.1.1.1:53";
                if !args.skip_network {
                    match tokio::net::TcpStream::connect(dns_server).await {
                        Ok(_) => {
                            info!("Public DNS server {dns_server} is reachable");
                            results.push(CheckStatus::Ok(format!(
                                "Public DNS server {dns_server} reachable"
                            )));
                        }
                        Err(e) => {
                            warn!("Cannot reach public DNS server {dns_server}: {e}");
                            results.push(CheckStatus::Warn(format!(
                                "Cannot reach public DNS server {dns_server}: {e}"
                            )));
                        }
                    }
                    use tokio::net::lookup_host;
                    match lookup_host("example.com:80").await {
                        Ok(_) => {
                            info!("DNS resolution seems OK");
                            results.push(CheckStatus::Ok("DNS resolution OK".into()));
                        }
                        Err(e) => {
                            warn!("Cannot resolve DNS (example.com): {e}");
                            results.push(CheckStatus::Warn(format!("DNS resolution failed: {e}")));
                        }
                    }
                } else {
                    info!("Skipping network checks as requested");
                }
            }
        }

        // === Integration Checks ===
        {
            let span = span!(Level::INFO, "integration");
            let _span_guard = span.enter();

            let app = PageshelfApp::from_server_config(sv_cfg, cfg);

            // Application-level checks
            app.run_checks().await;
            results.push(CheckStatus::Ok("Application self-checks passed".into()));
        }

        // === Summary ===
        info!("Checks completed.\n");

        let ok_count = results
            .iter()
            .filter(|r| matches!(r, CheckStatus::Ok(_)))
            .count();
        let warn_count = results
            .iter()
            .filter(|r| matches!(r, CheckStatus::Warn(_)))
            .count();
        let fail_count = results
            .iter()
            .filter(|r| matches!(r, CheckStatus::Fail(_)))
            .count();

        info!("========== SUMMARY ==========");
        for res in &results {
            info!("{}", res);
        }
        info!("==============================");
        info!("✅ OK: {ok_count} | ⚠️ WARN: {warn_count} | ❌ FAIL: {fail_count}");

        if fail_count > 0 {
            Err(eyre::eyre!("One or more checks failed"))
        } else {
            Ok(())
        }
    }
}
