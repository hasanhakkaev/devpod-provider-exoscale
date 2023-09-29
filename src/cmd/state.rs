use crate::exoscale::exoscale::ExoscaleProvider;
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[clap(name = "state", about = "State of an instance")]
pub struct State {}

impl State {
    pub async fn execute(&self) -> Result<()> {
        let hetzner = ExoscaleProvider::new_provider(false);
        match hetzner {
            Ok(provider) => {
                let status = provider.state().await;
                match status {
                    Err(err) => {
                        return Err(anyhow::anyhow!("Error getting instance state: {}", err))
                    }
                    _ => {}
                }
                println!("{}", status.unwrap());
            }
            Err(err) => return Err(err),
        }
        Ok(())
    }
}
