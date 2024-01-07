use exoscale_rs::models::instance_type::Size;
use ssh2::{Error, Session};
use std::io::Read;
use std::net::TcpStream;

pub async fn execute_command(
    user: String,
    ip: String,
    privatekey: String,
    command: String,
) -> Result<String, Error> {
    let mut sess = Session::new()?;

    let tcp = TcpStream::connect(format!("{}:22", ip)).unwrap();

    sess.set_tcp_stream(tcp);
    sess.handshake()?;

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        sess.userauth_pubkey_memory(&user, None, &privatekey, None)?;
    }
    #[cfg(target_os = "windows")]
    {
        sess.userauth_pubkey_file(&user, None, std::path::Path::new(&privatekey), None)?;
    }

    let mut channel = sess.channel_session()?;
    channel.exec(&command)?;

    let mut output = String::new();
    let mut buffer = [0; 1024];

    loop {
        let size = match channel.read(&mut buffer) {
            Ok(size) if size > 0 => {
                output.push_str(std::str::from_utf8(&buffer[..size]).unwrap());
                size
            }
            Ok(_) => break,
            Err(_err) => {
                let ssh2_err = Error::last_session_error(&sess).unwrap();
                return Err(ssh2_err);
            }
        };

        if size == 0 {
            break;
        }
    }

    Ok(output)
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
