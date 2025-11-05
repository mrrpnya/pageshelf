use color_eyre::eyre;
use config::Config;

use crate::{project::source::ProjectSource, provider::memory::MemoryProjectSource};

pub trait Plugin {
    /* ---------------------------------- Meta ---------------------------------- */

    fn title(&self) -> &str {
        "Untitled Plugin"
    }

    /* -------------------------------- Lifecycle ------------------------------- */

    async fn on_start(&self) {}

    /* -------------------------------- Provision ------------------------------- */

    async fn get_upstream(
        &self,
        name: &str,
        cfg: &Config,
    ) -> Result<Option<impl ProjectSource>, eyre::Report> {
        return Ok(None);

        Ok(Some(MemoryProjectSource::default()))
    }
}
