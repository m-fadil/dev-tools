use std::fmt;

#[derive(Debug)]
pub enum CliError {
    Usage(String),
    Failed(String),
    Cancelled,
    Exit(u8),
}

pub type Result<T> = std::result::Result<T, CliError>;

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Usage(m) | CliError::Failed(m) => f.write_str(m),
            CliError::Cancelled => f.write_str("Dibatalkan."),
            CliError::Exit(c) => write!(f, "exit {c}"),
        }
    }
}

impl std::error::Error for CliError {}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        CliError::Failed(e.to_string())
    }
}

impl From<inquire::InquireError> for CliError {
    fn from(e: inquire::InquireError) -> Self {
        match e {
            inquire::InquireError::OperationCanceled | inquire::InquireError::OperationInterrupted => {
                CliError::Cancelled
            }
            other => CliError::Failed(other.to_string()),
        }
    }
}

pub fn usage(msg: impl Into<String>) -> CliError {
    CliError::Usage(msg.into())
}
