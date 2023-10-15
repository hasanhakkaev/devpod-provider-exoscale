use crate::exoscale::exoscale::ExoscaleProvider;
use crate::ssh;
use anyhow::Result;
use clap::Parser;
use std::env;

#[derive(Parser)]
#[clap(name = "command", about = "Command an instance")]
pub struct Command {}

impl Command {
    pub async fn execute(&self) -> Result<()> {
        let exoscale = ExoscaleProvider::new_provider(false);
        match exoscale {
            Ok(provider) => {
                let command = env::var("COMMAND");
                let private_key;
                #[cfg(any(target_os = "linux", target_os = "macos"))]
                {
                    private_key = ssh::keys::get_private_key_raw_base(
                        provider.options.machine_folder.clone(),
                    );
                }
                #[cfg(target_os = "windows")]
                {
                    private_key = ssh::keys::get_private_key_filename(
                        provider.options.machine_folder.clone(),
                    );
                }
                let instance = provider.get_devpod_instance().await?;

                let result = ssh::helper::new_ssh_client(
                    "devpod".to_string(),
                    instance.public_ip.unwrap().clone(),
                    private_key.clone(),
                    command.unwrap(),
                )
                .await;
                match result {
                    Err(err) => return Err(anyhow::anyhow!("Error creating ssh client: {}", err)),
                    _ => {
                        println!("{}", result.unwrap());
                    }
                }
            }
            Err(err) => return Err(err),
        };
        Ok(())
    }
}
