#[derive(Debug, Error)]
pub enum CliError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("IoError: {0}")]
    IoError(#[from] std::io::Error),

    #[error("TomlSerError: {0}")]
    TomlSerError(#[from] toml::ser::Error),

    #[error("TomlDeError: {0}")]
    TomlDeError(#[from] toml::de::Error),
}
