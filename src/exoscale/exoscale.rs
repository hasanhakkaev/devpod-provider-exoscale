use crate::options::options::{from_env, Options};
use crate::ssh::helper::map_str_to_size;
use crate::ssh::keys;
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use exoscale_rs::apis::configuration::Configuration;
use exoscale_rs::apis::instance_api::{
    CreateInstanceParams, DeleteInstanceParams, ListInstancesParams, StartInstanceParams,
    StopInstanceParams,
};
use exoscale_rs::apis::security_group_api::{
    AddExternalSourceToSecurityGroupParams, AddRuleToSecurityGroupParams,
    CreateSecurityGroupParams, DeleteSecurityGroupParams, GetSecurityGroupParams,
};
use exoscale_rs::apis::template_api::ListTemplatesParams;
use exoscale_rs::models::security_group_resource::Visibility;
use exoscale_rs::models::start_instance_request::RescueProfile::NetbootEfi;
use exoscale_rs::models::{
    AddExternalSourceToSecurityGroupRequest, AddRuleToSecurityGroupRequest,
    CreateSecurityGroupRequest, InstanceType, ListInstances200ResponseInstancesInner,
    SecurityGroup, SecurityGroupResource, StartInstanceRequest, Template,
};
use std::collections::HashMap;
use std::env;
use std::thread::sleep;
use std::time::Duration;
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
        let zone =
            env::var("EXOSCALE_ZONE").context("Please set EXOSCALE_ZONE environment variable");

        let mut configuration = Configuration::new(zone.as_ref().unwrap());
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
        match zone {
            Ok(zone) => {
                configuration.zone = zone;
            }
            Err(err) => return Err(anyhow::anyhow!("Error getting ZONE: {}", err)),
        }
        let provider = ExoscaleProvider {
            configuration,
            options,
        };

        Ok(provider)
    }

    pub async fn get_devpod_instance(&self) -> Result<Box<ListInstances200ResponseInstancesInner>> {
        let instances = match exoscale_rs::apis::instance_api::list_instances(
            &self.configuration,
            ListInstancesParams {
                ip_address: None,
                manager_id: None,
                manager_type: None,
            },
        )
        .await
        {
            Ok(instances) => instances,
            Err(err) => return Err(anyhow::anyhow!("Error getting instance list: {}", err)),
        };

        let instance_list = match instances.instances {
            Some(instance_list) => instance_list,
            None => return Err(anyhow::anyhow!("No instance list found")),
        };

        if instance_list.is_empty() {
            return Err(anyhow::anyhow!("No instance found"));
        }
        let found_instance = instance_list
            .iter()
            .find(|instance| {
                if let (Some(_labels), Some(devpod_instance), Some(devpod_instance_id)) = (
                    &instance.labels,
                    instance
                        .labels
                        .as_ref()
                        .and_then(|l| l.get("devpod_instance")),
                    instance
                        .labels
                        .as_ref()
                        .and_then(|l| l.get("devpod_instance_id")),
                ) {
                    *devpod_instance == "true"
                        && *devpod_instance_id == self.options.machine_id.clone()
                } else {
                    false
                }
            })
            .cloned();

        if let Some(instance) = found_instance {
            Ok(Box::new(instance))
        } else {
            Err(anyhow::anyhow!("No matching instance found"))
        }
    }

    pub async fn init(&self) -> Result<()> {
        let _list = exoscale_rs::apis::zone_api::list_zones(&self.configuration).await?;
        Ok(())
    }

    pub async fn delete(&self) -> Result<()> {
        let devpod_instance = self.get_devpod_instance().await?;
        let instance_id: Option<Uuid> = devpod_instance.id;
        let sg_id: String = devpod_instance
            .security_groups
            .unwrap()
            .first()
            .unwrap()
            .id
            .unwrap()
            .to_string();

        // Delete the instance
        exoscale_rs::apis::instance_api::delete_instance(
            &self.configuration,
            DeleteInstanceParams {
                id: instance_id.as_ref().unwrap().to_string(),
            },
        )
        .await?;

        sleep(Duration::from_secs(10));
        //Delete security group
        exoscale_rs::apis::security_group_api::delete_security_group(
            &self.configuration,
            DeleteSecurityGroupParams { id: sg_id },
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

    pub async fn status(&self) -> Result<&str> {
        let devpod_instance = self.get_devpod_instance().await?;

        let status = match devpod_instance.state.unwrap().to_string().as_str() {
            "running" => "Running",
            "stopped" => "Stopped",
            "notfound" => "NotFound",
            "busy" => "Busy",
            _ => "NotFound",
        };

        Ok(status)
    }

    pub async fn create(&self) -> Result<()> {
        let public_key_base = keys::get_public_key_base(self.options.machine_folder.clone());

        let sg_result = exoscale_rs::apis::security_group_api::create_security_group(
            &self.configuration,
            CreateSecurityGroupParams {
                create_security_group_request: CreateSecurityGroupRequest {
                    name: self.options.machine_id.clone().to_string() + "-sg",
                    description: Some("Security group for devpod instance".to_string()),
                },
            },
        )
        .await;
        if let Err(err) = sg_result {
            return Err(anyhow::anyhow!("Error creating security group: {}", err));
        }

        let security_group_resource: Box<SecurityGroupResource> = Box::new(SecurityGroupResource {
            id: sg_result.as_ref().unwrap().reference.as_ref().unwrap().id,
            name: Some(self.options.machine_id.clone().to_string() + "-sg"),
            visibility: Option::from(Visibility::Private),
        });

        let _s = exoscale_rs::apis::security_group_api::add_external_source_to_security_group(
            &self.configuration,
            AddExternalSourceToSecurityGroupParams {
                id: security_group_resource.id.unwrap().to_string(),
                add_external_source_to_security_group_request:
                    AddExternalSourceToSecurityGroupRequest {
                        cidr: "0.0.0.0/0".to_string(),
                    },
            },
        )
        .await;
        if let Err(err) = _s {
            return Err(anyhow::anyhow!("Error creating security group: {}", err));
        }

        let _r =
            exoscale_rs::apis::security_group_api::add_rule_to_security_group(
                &self.configuration,
                AddRuleToSecurityGroupParams {
                    id: security_group_resource.id.unwrap().to_string(),
                    add_rule_to_security_group_request: AddRuleToSecurityGroupRequest {
                        description: Some("desc".to_string()),
                        start_port: Some(22),
                        end_port: Some(22),
                        flow_direction: exoscale_rs::models::add_rule_to_security_group_request::FlowDirection::Ingress,
                        icmp: None,
                        network:None,
                        protocol: exoscale_rs::models::add_rule_to_security_group_request::Protocol::Tcp,
                        security_group: Some(security_group_resource),
                    },
                },
            ).await;
        if let Err(err) = _r {
            return Err(anyhow::anyhow!(
                "Error adding rule to a security group: {}",
                err
            ));
        }

        let security_group = exoscale_rs::apis::security_group_api::get_security_group(
            &self.configuration,
            GetSecurityGroupParams {
                id: sg_result
                    .as_ref()
                    .unwrap()
                    .clone()
                    .reference
                    .as_ref()
                    .unwrap()
                    .id
                    .unwrap()
                    .to_string(),
            },
        )
        .await;
        if let Err(err) = security_group {
            return Err(anyhow::anyhow!("Error getting the security group: {}", err));
        }

        let sg = security_group.unwrap().clone();

        // Listing templates
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

        // Listing instance types
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
                    if map_str_to_size(self.options.instance_type.as_str()) == instance_type.size {
                        Some(instance_type.clone())
                    } else {
                        None
                    }
                })
                .unwrap(),
        );

        // Creating labels
        let mut labels = HashMap::new();
        labels.insert("devpod_instance".to_string(), "true".to_string());
        labels.insert(
            "devpod_instance_id".to_string(),
            self.options.machine_id.clone(),
        );

        let user_data = general_purpose::STANDARD.encode(format!(
            r#"#cloud-config
            users:
            - name: devpod
              shell: /bin/bash
              groups: [ sudo, docker ]
              ssh_authorized_keys:
              - {}
              sudo: [ "ALL=(ALL) NOPASSWD:ALL" ]"#,
            public_key_base
        ));

        // Constructing the request parameters for Instance creation
        let instance_request = exoscale_rs::models::CreateInstanceRequest {
            anti_affinity_groups: None,
            instance_type,
            template,
            disk_size: self.options.disk_size.parse().unwrap(),
            labels: Some(labels),
            auto_start: Option::from(true),
            security_groups: Some(vec![SecurityGroup {
                name: sg.name,
                id: sg.id,
                description: sg.description,
                external_sources: sg.external_sources,
                rules: None,
            }]),
            user_data: Some(user_data),
            deploy_target: None,
            public_ip_assignment: Some(exoscale_rs::models::PublicIpAssignment::Inet4),
            name: Some(self.options.machine_id.clone().to_string()),
            ssh_key: None,
            ipv6_enabled: None,
            ssh_keys: None,
        };

        let instance_create_params = CreateInstanceParams {
            create_instance_request: instance_request,
        };

        // Creating the instance
        let instance = exoscale_rs::apis::instance_api::create_instance(
            &self.configuration,
            instance_create_params,
        )
        .await;
        if let Err(err) = instance {
            return Err(anyhow::anyhow!("Error getting instance type list: {}", err));
        }
        Ok(())
    }
}
