mod message;
use crate::message::*;

mod config;
use crate::config::*;

use std::io::prelude::*;
use std::net::TcpStream;

fn main() {
    let config = load_config_from_env().unwrap();
    if let Err(error) = run(config) {
        let message = match error {
            RuntimeError::IoError(error) => format!("{:?}", error),
            RuntimeError::ParseMessageError => "ParseMessageError".to_string(),
            RuntimeError::TlsHandshakeError(error) => format!("{:?}", error),
            RuntimeError::TlsError(error) => format!("{:?}", error),
        };
        println!("Error: {}", message);
        std::process::exit(1);
    }
}

fn run(config: Config) -> Result<(), RuntimeError> {
    let addr = format!("{}:{}", config.host, config.port);

    let mut client = TcpStream::connect(addr.clone())?;

    client.write_all(&FrontendMessage::RequestSSL.to_bytes())?;

    let mut buf = [0u8; 1];
    client.read(&mut buf)?;

    assert!(buf == "S".as_bytes());

    let mut client = native_tls::TlsConnector::new()?.connect(&config.host, client)?;

    write_message(
        &mut client,
        FrontendMessage::StartupMessage {
            user: config.user.clone(),
            database: config.database.clone(),
        },
    )?;

    let mut requested = false;

    loop {
        let message = read_message(&mut client)?;

        match message {
            BackendMessage::AuthenticationCleartextPassword => write_message(
                &mut client,
                FrontendMessage::PasswordMessage {
                    password: config.password.clone(),
                },
            )?,
            BackendMessage::ReadyForQuery {
                status: ReadyForQueryStatus::Idle,
            } => {
                if !requested {
                    write_message(
                        &mut client,
                        FrontendMessage::SimpleQuery {
                            query: "SELECT * FROM my_table LIMIT 3;".to_string(),
                        },
                    )?;
                    requested = true;
                } else {
                    return Ok(());
                }
            }
            _ => {}
        }
    }
}

enum RuntimeError {
    IoError(std::io::Error),
    ParseMessageError,
    TlsHandshakeError(native_tls::HandshakeError<TcpStream>),
    TlsError(native_tls::Error),
}

impl From<std::io::Error> for RuntimeError {
    fn from(error: std::io::Error) -> Self {
        Self::IoError(error)
    }
}

impl From<ReadMessageError> for RuntimeError {
    fn from(error: ReadMessageError) -> Self {
        match error {
            ReadMessageError::IoError(error) => Self::IoError(error),
            ReadMessageError::ParseError => Self::ParseMessageError,
        }
    }
}

impl From<native_tls::HandshakeError<TcpStream>> for RuntimeError {
    fn from(error: native_tls::HandshakeError<TcpStream>) -> Self {
        RuntimeError::TlsHandshakeError(error)
    }
}

impl From<native_tls::Error> for RuntimeError {
    fn from(error: native_tls::Error) -> Self {
        RuntimeError::TlsError(error)
    }
}
