#[derive(Debug, Fail)]
pub enum CliError {
    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),
}
