use anyhow::Result;
use clap::Parser;

use crate::exoscale::exoscale::ExoscaleProvider;

#[derive(Parser)]
#[clap(name = "start", about = "Start an instance")]
pub struct Start {}

impl Start {
    pub async fn execute(&self) -> Result<()> {
        let exoscale = ExoscaleProvider::new_provider(false);
        match exoscale {
            Ok(provider) => {
                let start = provider.start().await;
                if let Err(err) = start {
                    return Err(anyhow::anyhow!("Error starting instance: {}", err));
                }
            }
            Err(err) => return Err(err),
        };
        Ok(())
    }
}
