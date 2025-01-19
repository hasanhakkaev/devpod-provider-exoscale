use crate::exoscale::exoscale::ExoscaleProvider;
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[clap(name = "status", about = "Status of an instance")]
pub struct Status {}

impl Status {
    pub async fn execute(&self) -> Result<()> {
        let exoscale = ExoscaleProvider::new_provider(false);
        match exoscale {
            Ok(provider) => {
                let status = provider.status().await;
                if let Err(err) = status {
                    return Err(anyhow::anyhow!("Error getting instance status: {}", err));
                }
                println!("{}", status?);
            }
            Err(err) => return Err(err),
        }
        Ok(())
    }
}
