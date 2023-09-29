use exoscale::models::instance_type::Size;
use exoscale::models::ZoneName;
use ssh2::{Error, ErrorCode, Session};
use std::io;
use std::io::{Read, Write};
use std::net::TcpStream;

pub async fn new_ssh_client(
    user: String,
    ip: String,
    privatekey: String,
    command: String,
) -> Result<String, Error> {
    if let Ok(stream) = TcpStream::connect(format!("{}:22", ip)) {
        let mut sess = Session::new().unwrap();
        sess.set_tcp_stream(stream);
        sess.handshake().unwrap();

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            sess.userauth_pubkey_memory(&user, None, &privatekey, None)
                .unwrap();
        }
        #[cfg(target_os = "windows")]
        {
            sess.userauth_pubkey_file(&user, None, std::path::Path::new(&privatekey), None)
                .unwrap();
        }
        let mut channel = sess.channel_session().unwrap();
        channel.exec(command.as_str()).unwrap();

        let mut buffer = [0; 100 * 1024];
        let mut stdin = io::stdin();
        let mut stdout = io::stdout();

        loop {
            let size2 = match channel.stderr().read(&mut buffer) {
                Ok(size2) => size2,
                Err(_) => break,
            };
            if size2 > 0 {
                eprint!("{}", std::str::from_utf8(&buffer[..size2]).unwrap());
            }

            let size = match channel.read(&mut buffer) {
                Ok(size) => size,
                Err(_) => break,
            };
            if size > 0 {
                let data = std::str::from_utf8(&buffer[..size]).unwrap();
                if data.contains("ping") {
                    stdout.write_all(&buffer[..size]).unwrap();
                    stdin.read_exact(&mut buffer[..size]).unwrap();
                    channel.write_all(&buffer[..size]).unwrap();
                } else {
                    stdout.write_all(&buffer[..size]).unwrap();
                }
            } else {
                channel.send_eof().unwrap();
                channel.wait_close().unwrap();
                break;
            }
        }
        Ok("".to_string())
    } else {
        return Err(Error::new(
            ErrorCode::Session(0),
            "Error connecting to ssh server",
        ));
    }
}

pub fn execute_command(command: String, sess: Session) -> Result<String, Error> {
    Ok("".to_string())
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
pub fn map_str_to_zone_name(zone_name_str: &str) -> Option<ZoneName> {
    match zone_name_str {
        "ch-dk-2" => Some(ZoneName::ChDk2),
        "de-muc-1" => Some(ZoneName::DeMuc1),
        "ch-gva-2" => Some(ZoneName::ChGva2),
        "at-vie-1" => Some(ZoneName::AtVie1),
        "de-fra-1" => Some(ZoneName::DeFra1),
        "bg-sof-1" => Some(ZoneName::BgSof1),
        _ => None,
    }
}
