use anyhow::Result;
use clap::Parser;

use crate::exoscale::exoscale::ExoscaleProvider;

#[derive(Parser)]
#[clap(name = "init", about = "Init account")]
pub struct Init {}

impl Init {
    pub async fn execute(&self) -> Result<()> {
        let hetzner = ExoscaleProvider::new_provider(true);
        match hetzner {
            Ok(provider) => provider.init().await,
            Err(err) => return Err(err),
        }
        .expect("TODO: panic message");
        Ok(())
    }
}
