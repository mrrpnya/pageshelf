use clap::Parser;
use color_eyre::eyre;
use pageshelf::conf::ServerConfig;

use crate::{app::PageshelfApp, cmd::Cli};

#[derive(Parser, Debug)]
#[command(about = "Begin serving web requests (run this for main functionality)")]
pub struct CmdServeArgs {}

impl Cli {
    pub async fn cmd_serve(&self, args: &CmdServeArgs) -> Result<(), eyre::Report> {
        let cfg = self.get_config().await?;
        let sv_cfg =
            ServerConfig::from_config(&cfg).expect("No server config available; This is required.");

        let app = PageshelfApp::from_server_config(sv_cfg);

        app.run(false).await;

        Ok(())
    }
}
