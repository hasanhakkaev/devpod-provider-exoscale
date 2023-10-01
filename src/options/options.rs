use std::env;

#[derive(Default)]
pub struct Options {
    pub template: String,
    pub instance_type: String,
    pub disk_size: String,
    pub machine_id: String,
    pub machine_folder: String,
}

pub fn from_env(init: bool) -> Options {
    let template = from_env_or_error("TEMPLATE");

    let instance_type = from_env_or_error("INSTANCE_TYPE");

    let disk_size = from_env_or_error("DISK_SIZE");

    if init {
        return Options {
            template,
            instance_type,
            disk_size,
            ..Default::default()
        };
    }
    let mut machine_id = from_env_or_error("MACHINE_ID");
    machine_id = machine_id.to_string();
    let machine_folder = from_env_or_error("MACHINE_FOLDER");
    Options {
        template,
        instance_type,
        disk_size,
        machine_id,
        machine_folder,
    }
}

fn from_env_or_error(name: &str) -> String {
    let value = env::var(name);
    match value {
        Ok(value) => value,
        Err(err) => panic!("Error reading {} from environment: {}", name, err),
    }
}
