#[derive(Debug, Error)]
pub enum CliError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),
}
