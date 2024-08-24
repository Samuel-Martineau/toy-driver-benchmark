use std::{
    env::{self, VarError},
    num::ParseIntError,
};

#[derive(Debug)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub database: String,
    pub password: String,
}

#[derive(Debug)]
pub enum ConfigParseError {
    VarError(VarError),
    ParseIntError(ParseIntError),
}

impl From<VarError> for ConfigParseError {
    fn from(error: VarError) -> Self {
        Self::VarError(error)
    }
}

impl From<ParseIntError> for ConfigParseError {
    fn from(error: ParseIntError) -> Self {
        Self::ParseIntError(error)
    }
}

pub fn load_config_from_env() -> Result<Config, ConfigParseError> {
    Ok(Config {
        host: env::var("HOST")?,
        port: env::var("PORT").map(|p| p.parse::<u16>())??,
        user: env::var("USER")?,
        database: env::var("DATABASE")?,
        password: env::var("PASSWORD")?,
    })
}
