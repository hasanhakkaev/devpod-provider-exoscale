use crate::options::options::{from_env, Options};
use crate::ssh::helper::map_str_to_size;
use crate::ssh::keys;
use anyhow::{Context, Result};
use exoscale_rs::apis::configuration::Configuration;
use exoscale_rs::apis::instance_api::{
    CreateInstanceParams, DeleteInstanceParams, ListInstancesParams, StartInstanceParams,
    StopInstanceParams,
};
use exoscale_rs::apis::template_api::ListTemplatesParams;
use exoscale_rs::models::start_instance_request::RescueProfile::NetbootEfi;
use exoscale_rs::models::{
    InstanceType, ListInstances200ResponseInstancesInner, StartInstanceRequest, Template,
};
use std::collections::HashMap;
use std::env;
use uuid::Uuid;

pub struct ExoscaleProvider {
    configuration: Configuration,
    pub options: Options,
}

impl ExoscaleProvider {
    pub fn new_provider(init: bool) -> Result<ExoscaleProvider> {
        let api_key = env::var("EXOSCALE_API_KEY")
            .context("Please set EXOSCALE_API_KEY environment variable");
        let api_secret = env::var("EXOSCALE_API_SECRET")
            .context("Please set EXOSCALE_API_SECRET environment variable");

        let mut configuration = Configuration::new();
        let options = from_env(init);

        match api_key {
            Ok(api_key) => {
                configuration.api_key = api_key;
            }
            Err(err) => return Err(anyhow::anyhow!("Error getting API key: {}", err)),
        }
        match api_secret {
            Ok(api_secret) => {
                configuration.api_secret = api_secret;
            }
            Err(err) => return Err(anyhow::anyhow!("Error getting API secret: {}", err)),
        }
        let provider = ExoscaleProvider {
            configuration,
            options,
        };

        Ok(provider)
    }

    pub async fn get_devpod_instance(&self) -> Result<Box<ListInstances200ResponseInstancesInner>> {
        let instances = exoscale_rs::apis::instance_api::list_instances(
            &self.configuration,
            ListInstancesParams {
                ip_address: None,
                manager_id: None,
                manager_type: None,
            },
        )
        .await;
        if let Err(err) = instances {
            return Err(anyhow::anyhow!("Error getting instance list: {}", err));
        }
        let instance_list = instances.unwrap().instances;
        if instance_list.is_none() {
            return Err(anyhow::anyhow!("No instance found"));
        }

        let instance: Box<ListInstances200ResponseInstancesInner> = Box::new(
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
        let _list = exoscale_rs::apis::instance_api::list_instances(
            &self.configuration,
            ListInstancesParams {
                ip_address: None,
                manager_id: None,
                manager_type: None,
            },
        )
        .await?;
        Ok(())
    }

    pub async fn delete(&self) -> Result<()> {
        let devpod_instance = self.get_devpod_instance().await?;
        let id: Option<Uuid> = devpod_instance.id;
        exoscale_rs::apis::instance_api::delete_instance(
            &self.configuration,
            DeleteInstanceParams {
                id: id.as_ref().unwrap().to_string(),
            },
        )
        .await?;
        Ok(())
    }

    pub async fn start(&self) -> Result<()> {
        let devpod_instance = self.get_devpod_instance().await?;
        let id: Option<Uuid> = devpod_instance.id;
        exoscale_rs::apis::instance_api::start_instance(
            &self.configuration,
            StartInstanceParams {
                id: id.as_ref().unwrap().to_string(),
                start_instance_request: StartInstanceRequest {
                    rescue_profile: Option::from(NetbootEfi),
                },
            },
        )
        .await?;
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        let devpod_instance = self.get_devpod_instance().await?;
        let id: Option<Uuid> = devpod_instance.id;
        exoscale_rs::apis::instance_api::stop_instance(
            &self.configuration,
            StopInstanceParams {
                id: id.as_ref().unwrap().to_string(),
            },
        )
        .await?;
        Ok(())
    }

    pub async fn state(&self) -> Result<String> {
        let devpod_instance = self.get_devpod_instance().await?;
        if devpod_instance.state == Option::from(exoscale_rs::models::InstanceState::Running) {
            Ok("Running".to_string())
        } else if devpod_instance.state == Option::from(exoscale_rs::models::InstanceState::Stopped)
        {
            Ok("Stopped".to_string())
        } else {
            Ok("Error".to_string())
        }
    }

    pub async fn create(&self) -> Result<()> {
        let public_key_base = keys::get_public_key_base(self.options.machine_folder.clone());

        let templates = exoscale_rs::apis::template_api::list_templates(
            &self.configuration,
            ListTemplatesParams {
                family: None,
                visibility: None,
            },
        )
        .await;
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
                    if template.name.as_ref().unwrap() == self.options.template.as_str() {
                        Some(template.clone())
                    } else {
                        None
                    }
                })
                .unwrap(),
        );

        let instance_types =
            exoscale_rs::apis::instance_type_api::list_instance_types(&self.configuration).await;
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

        let request_params = exoscale_rs::models::CreateInstanceRequest {
            anti_affinity_groups: None,
            instance_type,
            template,
            disk_size: self.options.disk_size.parse().unwrap(),
            labels: Some(labels),
            auto_start: None,
            security_groups: None,
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
            deploy_target: None,
            public_ip_assignment: None,
            name: None,
            ssh_key: None,
            ipv6_enabled: None,
            ssh_keys: None,
        };

        let _ = exoscale_rs::apis::instance_api::create_instance(
            &self.configuration,
            CreateInstanceParams {
                create_instance_request: request_params,
            },
        )
        .await?;
        /*
        let mut still_creating = true;

        while still_creating {
            let instance = exoscale_rs::apis::instance_api::get_instance(
                &self.configuration,
                result.id.as_ref().unwrap(),
            )
            .await?
            .clone();

            if instance.state == Option::from(exoscale_rs::models::InstanceState::Running) {
                still_creating = false;
            } else {
                std::thread::sleep(std::time::Duration::from_secs(5));
            }
        }*/
        Ok(())
    }
}
