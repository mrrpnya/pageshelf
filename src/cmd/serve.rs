use clap::Parser;
use color_eyre::eyre;
use pageshelf::conf::ServerConfig;

use crate::{app::PageshelfApp, cmd::Cli};

#[derive(Parser, Debug)]
#[command(about = "Begin serving web requests (run this for main functionality)")]
pub struct CmdServeArgs {
    /// Override host to bind to (default from config)
    #[arg(long)]
    pub host: Option<String>,
    /// Override port to bind to (default from config)
    #[arg(long)]
    pub port: Option<u16>,
    // Use a local content directory as a source (path to repo root or content dir)
    // TODO: Implement a provider later
    // #[arg(long)]
    // pub content_dir: Option<std::path::PathBuf>,
}

impl Cli {
    pub async fn cmd_serve(&self, args: &CmdServeArgs) -> Result<(), eyre::Report> {
        let cfg = self.get_config().await?;
        let sv_cfg =
            ServerConfig::from_config(&cfg).expect("No server config available; This is required.");

        let host_override = args.host.as_deref();
        let port_override = args.port;

        let app = PageshelfApp::from_server_config(sv_cfg);

        app.run(false, host_override, port_override).await?;

        Ok(())
    }
}
