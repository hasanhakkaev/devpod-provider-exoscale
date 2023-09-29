use crate::options::options::{from_env, Options};
use crate::ssh::helper::{map_str_to_size, map_str_to_zone_name};
use crate::ssh::keys;
use anyhow::{Context, Result};
use exoscale::apis::configuration::Configuration;
use exoscale::models::{Instance, InstanceType, Template};
use std::collections::HashMap;
use std::env;

pub struct ExoscaleProvider {
    configuration: Configuration,
    pub options: Options,
}

impl ExoscaleProvider {
    pub fn new_provider(init: bool) -> Result<ExoscaleProvider> {
        let key = env::var("EXOSCALE_API_KEY")
            .context("Please set EXOSCALE_API_KEY environment variable");
        let secret = env::var("EXOSCALE_API_SECRET")
            .context("Please set EXOSCALE_API_SECRET environment variable");

        let mut configuration = Configuration::new();
        let options = from_env(init);

        match key {
            Ok(key) => {
                configuration.api_key = exoscale::apis::configuration::ApiKey { key, prefix: None }
            }
            Err(err) => return Err(anyhow::anyhow!("Error getting API key: {}", err)),
        }
        match secret {
            Ok(secret) => {
                configuration.secret = secret;
            }
            Err(err) => return Err(anyhow::anyhow!("Error getting API secret: {}", err)),
        }
        let provider = ExoscaleProvider {
            configuration,
            options,
        };

        Ok(provider)
    }

    pub async fn get_devpod_instance(&self) -> Result<Box<Instance>> {
        let instances =
            exoscale::apis::instance_api::list_instances(&self.configuration, None, None).await;
        match instances {
            Err(err) => return Err(anyhow::anyhow!("Error getting instance list: {}", err)),
            _ => {}
        }
        let instance_list = instances.unwrap().instances;
        if instance_list.is_none() {
            return Err(anyhow::anyhow!("No instance found"));
        }

        let instance: Box<Instance> = Box::new(
            instance_list
                .unwrap()
                .iter()
                .find_map(|instance| {
                    if instance
                        .labels
                        .as_ref()
                        .unwrap()
                        .get(self.options.machine_id.as_str())
                        .is_some()
                    {
                        Some(instance.clone())
                    } else {
                        None
                    }
                })
                .unwrap(),
        );
        Ok(instance)
    }

    pub async fn init(&self) -> Result<()> {
        let _list =
            exoscale::apis::instance_api::list_instances(&self.configuration, None, None).await?;
        Ok(())
    }

    pub async fn delete(&self) -> Result<()> {
        let devpod_instance = self.get_devpod_instance().await?;
        let id: Option<&str> = devpod_instance.id.as_deref();
        exoscale::apis::instance_api::delete_instance(&self.configuration, id.unwrap()).await?;
        Ok(())
    }

    pub async fn start(&self) -> Result<()> {
        let devpod_instance = self.get_devpod_instance().await?;
        let id: Option<&str> = devpod_instance.id.as_deref();
        exoscale::apis::instance_api::start_instance(
            &self.configuration,
            id.unwrap(),
            exoscale::models::StartInstanceRequest {
                rescue_profile: Default::default(),
            },
        )
        .await?;
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        let devpod_instance = self.get_devpod_instance().await?;
        let id: Option<&str> = devpod_instance.id.as_deref();
        exoscale::apis::instance_api::stop_instance(&self.configuration, id.unwrap()).await?;
        Ok(())
    }

    pub async fn state(&self) -> Result<String> {
        let devpod_instance = self.get_devpod_instance().await?;
        if devpod_instance.state == Option::from(exoscale::models::instance::State::Running) {
            Ok("Running".to_string())
        } else if devpod_instance.state == Option::from(exoscale::models::instance::State::Stopped)
        {
            Ok("Stopped".to_string())
        } else {
            Ok("Error".to_string())
        }
    }

    pub async fn create(&self) -> Result<()> {
        let public_key_base = keys::get_public_key_base(self.options.machine_folder.clone());

        let templates =
            exoscale::apis::template_api::list_templates(&self.configuration, None, None).await;
        if let Err(err) = templates {
            return Err(anyhow::anyhow!("Error getting template list: {}", err));
        }
        let template_list = templates.unwrap().templates;
        if template_list.is_none() {
            return Err(anyhow::anyhow!("No template found"));
        }
        let template: Box<Template> = Box::new(
            template_list
                .unwrap()
                .iter()
                .find_map(|template| {
                    if template.name == Some(self.options.template.clone())
                        && template
                            .zones
                            .clone()
                            .unwrap()
                            .iter()
                            .find_map(|zone| {
                                if zone
                                    == &map_str_to_zone_name(self.options.zone.as_str()).unwrap()
                                {
                                    Some(*zone)
                                } else {
                                    None
                                }
                            })
                            .is_some()
                    {
                        Some(template.clone())
                    } else {
                        None
                    }
                })
                .unwrap(),
        );

        let instance_types =
            exoscale::apis::instance_type_api::list_instance_types(&self.configuration).await;
        if let Err(err) = instance_types {
            return Err(anyhow::anyhow!("Error getting instance type list: {}", err));
        }
        let instance_type_list = instance_types.unwrap().instance_types;
        if instance_type_list.is_none() {
            return Err(anyhow::anyhow!("No instance type found"));
        }

        let instance_type: Box<InstanceType> = Box::new(
            instance_type_list
                .unwrap()
                .iter()
                .find_map(|instance_type| {
                    if map_str_to_size(self.options.instance_type.as_str()).unwrap()
                        == instance_type.size.unwrap()
                    {
                        Some(instance_type.clone())
                    } else {
                        None
                    }
                })
                .unwrap(),
        );

        let mut labels = HashMap::new();
        labels.insert(self.options.machine_id.clone(), "true".to_string());

        let request_params = exoscale::models::CreateInstanceRequest {
            instance_type,
            template,
            disk_size: self.options.disk_size.parse().unwrap(),
            labels: Some(labels),
            user_data: Some(format!(
                r#"#cloud-config
users:
- name: devpod
  shell: /bin/bash
  groups: [ sudo, docker ]
  ssh_authorized_keys:
  - {}
  sudo: [ "ALL=(ALL) NOPASSWD:ALL" ]"#,
                public_key_base
            )),
            ..Default::default()
        };

        let result =
            exoscale::apis::instance_api::create_instance(&self.configuration, request_params)
                .await?;

        let mut still_creating = true;

        while still_creating {
            let instance = exoscale::apis::instance_api::get_instance(
                &self.configuration,
                result.id.as_ref().unwrap(),
            )
            .await?
            .clone();

            if instance.state == Option::from(exoscale::models::instance::State::Running) {
                still_creating = false;
            } else {
                std::thread::sleep(std::time::Duration::from_secs(5));
            }
        }
        Ok(())
    }
}
