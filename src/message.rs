use std::array::TryFromSliceError;
use std::collections::HashMap;
use std::io::prelude::*;
use std::str;

trait Encoder {
    fn process(self) -> Vec<u8>;
}

impl Encoder for u16 {
    fn process(self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl Encoder for u32 {
    fn process(self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl Encoder for &str {
    fn process(self) -> Vec<u8> {
        let mut bytes = self.as_bytes().to_vec();
        bytes.push(0);
        bytes
    }
}

impl Encoder for String {
    fn process(self) -> Vec<u8> {
        let mut bytes = (self).as_bytes().to_vec();
        bytes.push(0);
        bytes
    }
}

macro_rules! encode {
    ($prefix:expr, $($args:expr),*) => {{
        let mut result = vec![];
        $(result.extend(Encoder::process($args));)*
        // result.push(0);
        let size = Encoder::process(result.len() as u32 + 4);
        [$prefix.as_bytes().to_vec(), size, result].concat()
    }};
}

#[derive(Debug)]
pub enum FrontendMessage {
    RequestSSL,
    StartupMessage { user: String, database: String },
    PasswordMessage { password: String },
    SimpleQuery { query: String },
}

impl FrontendMessage {
    pub fn to_bytes(self) -> Vec<u8> {
        match self {
            Self::RequestSSL => encode!("", 1234u16, 5679u16, ""),
            Self::StartupMessage { user, database } => {
                encode!("", 3u16, 0u16, "user", user, "database", database, "")
            }
            Self::PasswordMessage { password } => encode!("p", password, ""),
            Self::SimpleQuery { query } => encode!("Q", query),
        }
    }
}

#[derive(Debug)]
pub enum ReadyForQueryStatus {
    Idle,
    Transaction,
    FailedTransaction,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum ErrorField {
    LocalizedSeverity,
    Severity,
    Code,
    Message,
    Detail,
    Hint,
    Position,
    InternalPosition,
    InternalQuery,
    Where,
    SchemaName,
    TableName,
    ColumnName,
    DataTypeName,
    ConstraintName,
    File,
    Line,
    Routine,
    Unknown(char),
}

#[derive(Debug)]
pub enum BackendMessage {
    AuthenticationOk,
    AuthenticationCleartextPassword,
    AuthenticationSasl { mechanisms: Vec<String> },
    ErrorResponse(HashMap<ErrorField, String>),
    BackendKeyData { process_id: u32, secret_key: i32 },
    ReadyForQuery { status: ReadyForQueryStatus },
    ParameterStatus { name: String, value: String },
    Unknown { prefix: char, payload: Vec<u8> },
}

pub fn read_message(reader: &mut dyn Read) -> Result<BackendMessage, ReadMessageError> {
    let mut prefix = [0u8; 1];
    reader.read_exact(&mut prefix)?;
    let prefix = char::from(prefix[0]);

    let mut length = [0u8; 4];
    reader.read_exact(&mut length)?;
    let length = u32::from_be_bytes(length);

    let mut body = vec![0u8; (length - 4).try_into()?];
    reader.read_exact(&mut body)?;

    let message = match (prefix, length, body) {
        ('R', 8, payload) if payload == [0, 0, 0, 3] => {
            BackendMessage::AuthenticationCleartextPassword
        }
        ('R', 8, payload) if payload[0..4] == [0, 0, 0, 0] => BackendMessage::AuthenticationOk,
        ('R', _, payload) if payload[0..4] == [0, 0, 0, 10] => {
            let mechanisms = str::from_utf8(&payload[4..payload.len() - 2])?
                .split('\0')
                .map(|s| s.to_string())
                .collect();
            BackendMessage::AuthenticationSasl { mechanisms }
        }
        ('E', _, payload) => {
            let error = str::from_utf8(&payload[..payload.len() - 2])?
                .split('\0')
                .map(|s| {
                    (
                        if let Some(char) = s.chars().nth(0) {
                            match char {
                                'S' => ErrorField::LocalizedSeverity,
                                'V' => ErrorField::Severity,
                                'C' => ErrorField::Code,
                                'M' => ErrorField::Message,
                                'D' => ErrorField::Detail,
                                'H' => ErrorField::Hint,
                                'P' => ErrorField::Position,
                                'p' => ErrorField::InternalPosition,
                                'q' => ErrorField::InternalQuery,
                                'W' => ErrorField::Where,
                                's' => ErrorField::SchemaName,
                                't' => ErrorField::TableName,
                                'c' => ErrorField::ColumnName,
                                'd' => ErrorField::DataTypeName,
                                'n' => ErrorField::ConstraintName,
                                'F' => ErrorField::File,
                                'L' => ErrorField::Line,
                                'R' => ErrorField::Routine,
                                c => ErrorField::Unknown(c),
                            }
                        } else {
                            ErrorField::Unknown('\0')
                        },
                        s[1..].to_string(),
                    )
                })
                .collect();
            BackendMessage::ErrorResponse(error)
        }
        ('K', 12, payload) => {
            let process_id = u32::from_be_bytes(payload[..4].try_into()?);
            let secret_key = i32::from_be_bytes(payload[4..].try_into()?);
            BackendMessage::BackendKeyData {
                process_id,
                secret_key,
            }
        }
        ('Z', 5, payload) => {
            let status = match char::from(payload[0]) {
                'I' => ReadyForQueryStatus::Idle,
                'T' => ReadyForQueryStatus::Transaction,
                'E' => ReadyForQueryStatus::FailedTransaction,
                _ => Err(ReadMessageError::ParseError)?,
            };
            BackendMessage::ReadyForQuery { status }
        }
        ('S', _, payload) => {
            let index = payload
                .iter()
                .position(|&x| x == 0)
                .ok_or(ReadMessageError::ParseError)?;

            BackendMessage::ParameterStatus {
                name: str::from_utf8(&payload[..index])?.to_string(),
                value: str::from_utf8(&payload[index + 1..payload.len() - 1])?.to_string(),
            }
        }
        (prefix, _, payload) => BackendMessage::Unknown { prefix, payload },
    };

    println!("<-- {:?}", message);

    Ok(message)
}

#[derive(Debug)]
pub enum ReadMessageError {
    IoError(std::io::Error),
    ParseError,
}

impl From<std::io::Error> for ReadMessageError {
    fn from(error: std::io::Error) -> Self {
        Self::IoError(error)
    }
}

impl From<std::num::TryFromIntError> for ReadMessageError {
    fn from(_error: std::num::TryFromIntError) -> Self {
        Self::ParseError
    }
}

impl From<str::Utf8Error> for ReadMessageError {
    fn from(_error: str::Utf8Error) -> Self {
        Self::ParseError
    }
}

impl From<TryFromSliceError> for ReadMessageError {
    fn from(_error: TryFromSliceError) -> Self {
        Self::ParseError
    }
}

pub fn write_message(
    writer: &mut dyn Write,
    message: FrontendMessage,
) -> Result<(), std::io::Error> {
    println!("--> {:?}", message);
    writer.write_all(&message.to_bytes())
}
