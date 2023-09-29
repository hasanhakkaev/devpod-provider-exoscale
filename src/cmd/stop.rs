use crate::exoscale::exoscale::ExoscaleProvider;
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[clap(name = "stop", about = "Stop an instance")]
pub struct Stop {}

impl Stop {
    pub async fn execute(&self) -> Result<()> {
        let exoscale = ExoscaleProvider::new_provider(false);
        match exoscale {
            Ok(provider) => {
                let stop = provider.stop().await;
                match stop {
                    Err(err) => return Err(anyhow::anyhow!("Error stopping instance: {}", err)),
                    _ => {}
                }
            }
            Err(err) => return Err(err),
        };
        Ok(())
    }
}
