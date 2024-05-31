use exoscale_rs::models::instance_type::Size;
use ssh2::Session;
use std::io::Read;
use std::net::TcpStream;
use std::time::Duration;
use thiserror::Error;
use tokio::task;
use tokio::time::sleep;

#[derive(Debug, Error)]
pub enum SshError {
    #[error("SSH session error: {0}")]
    Session(String),
    #[error("IO error: {0}")]
    Io(String),
    #[error("UTF-8 conversion error")]
    Utf8,
    #[error("Health check failed: Unable to connect to port 22 on {0}")]
    HealthCheckFailed(String),
}

impl From<ssh2::Error> for SshError {
    fn from(err: ssh2::Error) -> Self {
        SshError::Session(err.to_string())
    }
}

impl From<std::io::Error> for SshError {
    fn from(err: std::io::Error) -> Self {
        SshError::Io(err.to_string())
    }
}

// Async function to check port 22 availability with retries
async fn check_port(ip: &str, max_retries: u32, retry_delay: Duration) -> Result<(), SshError> {
    for attempt in 0..max_retries {
        match TcpStream::connect(format!("{}:22", ip)) {
            Ok(_) => return Ok(()),
            Err(_) if attempt < max_retries - 1 => {
                sleep(retry_delay).await;
            }
            Err(_) => return Err(SshError::HealthCheckFailed(ip.to_string())),
        }
    }
    Ok(())
}

pub async fn execute_command(
    user: String,
    ip: String,
    private_key: String,
    command: String,
) -> Result<String, SshError> {
    // Health check for port 22 with retry
    const MAX_RETRIES: u32 = 5;
    const RETRY_DELAY: Duration = Duration::from_secs(5);

    check_port(&ip, MAX_RETRIES, RETRY_DELAY).await?;

    task::spawn_blocking(move || {
        let mut session = Session::new().map_err(SshError::from)?;

        let tcp = TcpStream::connect(format!("{}:22", ip)).map_err(SshError::from)?;
        session.set_tcp_stream(tcp);
        session.handshake().map_err(SshError::from)?;

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            session
                .userauth_pubkey_memory(&user, None, &private_key, None)
                .map_err(SshError::from)?;
        }
        #[cfg(target_os = "windows")]
        {
            session
                .userauth_pubkey_file(&user, None, std::path::Path::new(&private_key), None)
                .map_err(SshError::from)?;
        }

        let mut channel = session.channel_session().map_err(SshError::from)?;
        channel.exec(&command).map_err(SshError::from)?;

        let mut output = String::new();
        let mut buffer = [0; 1024];

        loop {
            let size = channel.read(&mut buffer).map_err(SshError::from)?;

            if size == 0 {
                break;
            }

            output.push_str(std::str::from_utf8(&buffer[..size]).map_err(|_| SshError::Utf8)?);
        }

        Ok(output)
    })
    .await
    .map_err(|e| SshError::Io(e.to_string()))?
}

pub fn map_str_to_size(size_str: &str) -> Option<Size> {
    match size_str {
        "large" => Some(Size::Large),
        "huge" => Some(Size::Huge),
        "jumbo" => Some(Size::Jumbo),
        "medium" => Some(Size::Medium),
        "mega" => Some(Size::Mega),
        "small" => Some(Size::Small),
        "extra-large" => Some(Size::ExtraLarge),
        "titan" => Some(Size::Titan),
        "micro" => Some(Size::Micro),
        "colossus" => Some(Size::Colossus),
        "tiny" => Some(Size::Tiny),
        _ => None,
    }
}
