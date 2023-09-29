use anyhow::Result;
use clap::Parser;

use crate::exoscale::exoscale::ExoscaleProvider;

#[derive(Parser)]
#[clap(name = "create", about = "Create an instance")]
pub struct Create {}

impl Create {
    pub async fn execute(&self) -> Result<()> {
        let exoscale = ExoscaleProvider::new_provider(false);
        match exoscale {
            Ok(provider) => {
                let create = provider.create().await;
                match create {
                    Err(err) => return Err(anyhow::anyhow!("Error creating instance: {}", err)),
                    _ => {}
                }
            }
            Err(err) => return Err(err),
        }
        Ok(())
    }
}
