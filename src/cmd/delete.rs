use anyhow::Result;
use clap::Parser;

use crate::exoscale::exoscale::ExoscaleProvider;

#[derive(Parser)]
#[clap(name = "delete", about = "Delete an instance")]
pub struct Delete {}

impl Delete {
    pub async fn execute(&self) -> Result<()> {
        let exoscale = ExoscaleProvider::new_provider(false);
        match exoscale {
            Ok(provider) => {
                provider.delete().await?;
            }
            Err(err) => return Err(err),
        }
        Ok(())
    }
}
