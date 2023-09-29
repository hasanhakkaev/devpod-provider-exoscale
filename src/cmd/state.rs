use crate::exoscale::exoscale::ExoscaleProvider;
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[clap(name = "state", about = "State of an instance")]
pub struct State {}

impl State {
    pub async fn execute(&self) -> Result<()> {
        let exoscale = ExoscaleProvider::new_provider(false);
        match exoscale {
            Ok(provider) => {
                let status = provider.state().await;
                if let Err(err) = status {
                    return Err(anyhow::anyhow!("Error getting instance state: {}", err));
                }
                println!("{}", status.unwrap());
            }
            Err(err) => return Err(err),
        }
        Ok(())
    }
}
